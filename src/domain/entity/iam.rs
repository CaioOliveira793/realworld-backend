use url::Url;

use super::{state_ref, EntityCtl};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserState {
    pub(in crate::domain) username: String,
    pub(in crate::domain) email: String,
    pub(in crate::domain) password_hash: String,
    pub(in crate::domain) bio: Option<String>,
    pub(in crate::domain) image_url: Option<Url>,
}

pub type User = EntityCtl<UserState>;

impl User {
    state_ref!(username, String);
    state_ref!(email, String);
    state_ref!(password_hash, String);
    state_ref!(bio, Option<String>);
    state_ref!(image_url, Option<Url>);
}
