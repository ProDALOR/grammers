// Copyright 2020 - developers of the `grammers` project.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use grammers_session::{PackedChat, PackedType};
use std::collections::HashMap;

pub struct PeerCache {
    cache: HashMap<(PackedType, i64), i64>,
}

fn normalize_type(ty: PackedType) -> PackedType {
    match ty {
        PackedType::User | PackedType::Bot => PackedType::User,
        PackedType::Chat => PackedType::Chat,
        PackedType::Megagroup | PackedType::Broadcast | PackedType::Gigagroup => {
            PackedType::Broadcast
        }
    }
}

impl PeerCache {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    pub fn insert(&mut self, chat: PackedChat) {
        if let Some(hash) = chat.access_hash {
            self.cache.insert((normalize_type(chat.ty), chat.id), hash);
        }
    }

    pub fn get(&self, (ty, id): (PackedType, i64)) -> PackedChat {
        self.cache
            .get(&(normalize_type(ty), id))
            .map(|&hash| PackedChat {
                ty,
                id,
                access_hash: Some(hash),
            })
            .unwrap_or(PackedChat {
                ty,
                id,
                access_hash: None,
            })
    }
}
