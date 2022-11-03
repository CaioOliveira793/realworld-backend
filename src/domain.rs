pub mod entity {
    use chrono::{DateTime, Utc};
    use uuid::Uuid;

    pub trait Entity {
        fn ident(&self) -> Uuid;
        fn version(&self) -> u32;
        fn created(&self) -> DateTime<Utc>;
        fn updated(&self) -> Option<DateTime<Utc>>;
    }

    #[derive(Debug, Clone)]
    pub struct EntityCtl<State> {
        id: Uuid,
        created: DateTime<Utc>,
        updated: Option<DateTime<Utc>>,
        version: u32,
        state: State,
    }

    #[derive(Debug, Clone)]
    pub struct EntityData {
        pub id: Uuid,
        pub created: DateTime<Utc>,
        pub updated: Option<DateTime<Utc>>,
        pub version: u32,
    }

    impl<State> Entity for EntityCtl<State> {
        fn ident(&self) -> Uuid {
            self.id
        }

        fn version(&self) -> u32 {
            self.version
        }

        fn created(&self) -> DateTime<Utc> {
            self.created
        }

        fn updated(&self) -> Option<DateTime<Utc>> {
            self.updated
        }
    }

    impl<State> EntityCtl<State> {
        pub fn restore(ent: EntityData, state: State) -> Self {
            Self {
                state,
                id: ent.id,
                created: ent.created,
                updated: ent.updated,
                version: ent.version,
            }
        }

        pub fn new(state: State) -> Self {
            Self {
                id: Uuid::new_v4(),
                created: Utc::now(),
                updated: None,
                version: 1,
                state,
            }
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct UserState {
        pub(super) username: String,
        pub(super) email: String,
        pub(super) password_hash: String,
        pub(super) bio: Option<String>,
        pub(super) image_url: Option<String>,
    }

    pub type User = EntityCtl<UserState>;

    impl User {
        pub fn username(&self) -> String {
            self.state.username.clone()
        }

        pub fn email(&self) -> String {
            self.state.email.clone()
        }

        pub fn password_hash(&self) -> String {
            self.state.password_hash.clone()
        }

        pub fn bio(&self) -> Option<String> {
            self.state.bio.clone()
        }

        pub fn image_url(&self) -> Option<String> {
            self.state.image_url.clone()
        }
    }
}

mod transform {
    use crate::app::resource::iam::CreateUserDto;

    use super::entity::{EntityData, User, UserState};

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

    impl From<&tokio_postgres::Row> for UserState {
        fn from(row: &tokio_postgres::Row) -> Self {
            Self {
                email: row.get("email"),
                username: row.get("username"),
                password_hash: row.get("password_hash"),
                image_url: row.get("image_url"),
                bio: row.get("bio"),
            }
        }
    }

    impl From<&tokio_postgres::Row> for User {
        fn from(row: &tokio_postgres::Row) -> Self {
            Self::restore(row.into(), row.into())
        }
    }
}
