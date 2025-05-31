// Copyright 2020 - developers of the `grammers` project.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

mod peer_cache;

use grammers_client::session::Session;
use grammers_client::{Client, Config, SignInError};

use crate::bridge::{BackendContext, BackendMessage, FrontendMessage};
use crate::config;
use peer_cache::PeerCache;

pub async fn main(mut context: BackendContext) -> Result<(), Box<dyn std::error::Error>> {
    log::info!("connecting to Telegram...");
    let client = Client::connect(Config {
        session: Session::load_file_or_create(config::SESSION_FILE_NAME)?,
        api_id: config::TG_API_ID.parse().expect("TG_API_ID to be numeric"),
        api_hash: config::TG_API_HASH.to_owned(),
        params: Default::default(),
    })
    .await?;

    let mut peer_cache = PeerCache::new();
    let mut sign_out = false;

    if !client.is_authorized().await? {
        log::info!("not authorized; starting login flow...");

        let phone = loop {
            context
                .backend_sender
                .send(BackendMessage::NeedLoginPhone)?;
            match context.backend_receiver.recv().await {
                Some(FrontendMessage::LoginPhone(phone)) => break phone,
                Some(_) => continue,
                None => return Ok(()),
            }
        };

        log::debug!("requesting login for phone {phone}...");
        let token = client.request_login_code(&phone).await?;

        log::debug!("waiting for login code...");
        let code = loop {
            context.backend_sender.send(BackendMessage::NeedLoginCode)?;
            match context.backend_receiver.recv().await {
                Some(FrontendMessage::LoginCode(code)) => break code,
                Some(_) => continue,
                None => return Ok(()),
            }
        };

        log::debug!("attempting to login without password...");
        let signed_in = client.sign_in(&token, &code).await;

        match signed_in {
            Err(SignInError::PasswordRequired(password_token)) => {
                log::debug!("waiting for login password...");
                let password = loop {
                    context
                        .backend_sender
                        .send(BackendMessage::NeedLoginPassword {
                            hint: password_token.hint().map(|s| s.to_owned()),
                        })?;
                    match context.backend_receiver.recv().await {
                        Some(FrontendMessage::LoginPassword(password)) => break password,
                        Some(_) => continue,
                        None => return Ok(()),
                    }
                };

                log::debug!("attempting to login with password...");
                client
                    .check_password(password_token, password.trim())
                    .await?;
            }
            Ok(_) => (),
            Err(e) => return Err(e.into()),
        };

        log::info!("login success; saving session file...");
        match client.session().save_to_file(config::SESSION_FILE_NAME) {
            Ok(_) => {}
            Err(e) => {
                log::error!("saving session after login failed: {e}");
                sign_out = true;
            }
        }
    }

    context.backend_sender.send(BackendMessage::LoginSuccess)?;
    let mut dialogs_iter = client.iter_dialogs().limit(100);
    let mut dialogs = Vec::with_capacity(100);

    log::info!("fetching initial dialogs...");
    while let Some(dialog) = dialogs_iter.next().await? {
        peer_cache.insert(dialog.chat().pack());
        dialogs.push(dialog);
    }

    context
        .backend_sender
        .send(BackendMessage::Dialogs(dialogs))?;

    log::info!("starting loop to listen for frontend messages...");
    while let Some(message) = context.backend_receiver.recv().await {
        log::debug!("processing frontend message...");
        match message {
            FrontendMessage::LoginPhone(_)
            | FrontendMessage::LoginCode(_)
            | FrontendMessage::LoginPassword(_) => {
                context.backend_sender.send(BackendMessage::LoginSuccess)?;
            }
            FrontendMessage::FetchMessages { chat } => {
                let mut messages_iter = client.iter_messages(peer_cache.get(chat)).limit(100);
                let mut messages = Vec::with_capacity(100);
                while let Some(message) = messages_iter.next().await? {
                    messages.push(message);
                }
                context
                    .backend_sender
                    .send(BackendMessage::Messages(messages))?;
            }
            FrontendMessage::SendMessage { chat, message } => {
                client.send_message(peer_cache.get(chat), message).await?;
            }
        }
    }

    if sign_out {
        log::warn!("attempting to sign out since saving session after login failed...");
        client.sign_out_disconnect().await?;
    }

    Ok(())
}
