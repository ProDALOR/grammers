// Copyright 2020 - developers of the `grammers` project.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

slint::include_modules!();

use grammers_session::{PackedChat, PackedType};

impl From<PackedChat> for ChatId {
    fn from(value: PackedChat) -> Self {
        Self {
            r#type: match value.ty {
                PackedType::User | PackedType::Bot => ChatTypeEnum::User,
                PackedType::Chat => ChatTypeEnum::Chat,
                PackedType::Megagroup | PackedType::Broadcast | PackedType::Gigagroup => {
                    ChatTypeEnum::Channel
                }
            },
            hi: (value.id >> 32) as i32,
            lo: value.id as i32,
        }
    }
}

impl ChatId {
    pub fn to_tuple(&self) -> (PackedType, i64) {
        (
            match self.r#type {
                ChatTypeEnum::User => PackedType::User,
                ChatTypeEnum::Chat => PackedType::Chat,
                ChatTypeEnum::Channel => PackedType::Broadcast,
            },
            ((self.hi as i64) << 32) | (self.lo as i64),
        )
    }
}
