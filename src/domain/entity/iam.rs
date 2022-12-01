use url::Url;

use crate::domain::datatype::security::PasswordHash;

use super::{impl_entity, state_ref, transform_helper, EntityData};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserState {
    pub(in crate::domain) username: String,
    pub(in crate::domain) email: String,
    pub(in crate::domain) password_hash: PasswordHash,
    pub(in crate::domain) bio: Option<String>,
    pub(in crate::domain) image_url: Option<Url>,
}

#[derive(Debug)]
pub struct User {
    pub(in crate::domain) data: EntityData,
    pub(in crate::domain) state: UserState,
}

impl_entity!(User);

impl User {
    state_ref!(username, String);
    state_ref!(email, String);
    state_ref!(password_hash, PasswordHash);
    state_ref!(bio, Option<String>);
    state_ref!(image_url, Option<Url>);

    transform_helper!(UserState);

    pub fn new(email: String, username: String, password_hash: PasswordHash) -> Self {
        Self::restore(
            EntityData::new(),
            UserState {
                email,
                username,
                password_hash,
                bio: None,
                image_url: None,
            },
        )
    }
}
