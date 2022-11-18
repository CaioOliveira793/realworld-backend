use super::entity::{
    iam::{User, UserState},
    EntityData,
};
use crate::app::resource::iam::CreateUserDto;

impl From<&tokio_postgres::Row> for EntityData {
    fn from(row: &tokio_postgres::Row) -> Self {
        Self {
            id: row.get("id"),
            created: row.get("created"),
            updated: row.get("updated"),
            version: row.get("version"),
        }
    }
}

impl<'a> From<CreateUserDto<'a>> for UserState {
    fn from(dto: CreateUserDto<'a>) -> Self {
        Self {
            email: dto.email.into(),
            bio: None,
            image_url: None,
            password_hash: dto.password.into(),
            username: dto.username.into(),
        }
    }
}

impl From<&tokio_postgres::Row> for UserState {
    fn from(row: &tokio_postgres::Row) -> Self {
        Self {
            email: row.get("email"),
            username: row.get("username"),
            password_hash: row.get("password_hash"),
            image_url: row.get::<&str, Option<&str>>("image_url").map(|s| {
                s.parse()
                    .expect("user table to have image_url of type TEXT with valid url")
            }),
            bio: row.get("bio"),
        }
    }
}

impl From<&tokio_postgres::Row> for User {
    fn from(row: &tokio_postgres::Row) -> Self {
        Self::restore(row.into(), row.into())
    }
}
