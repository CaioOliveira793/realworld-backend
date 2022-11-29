use sqlx::{postgres::PgRow, FromRow, Row};

use super::entity::EntityData;

impl<'r> FromRow<'r, PgRow> for EntityData {
    fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
        Ok(Self {
            id: row.try_get("id")?,
            version: row.try_get::<i64, &str>("version")? as u32,
            created: row.try_get("created")?,
            updated: row.try_get("updated")?,
        })
    }
}

macro_rules! column_decode_error {
    ($table:literal, $column:literal, $type:literal) => {
        concat!(
            "Expect \"",
            $table,
            "\" table to have \"",
            $column,
            "\" column of type ",
            $type
        )
    };

    ($table:literal, $column:literal, $type:literal, $with:literal) => {
        concat!(
            column_decode_error!($table, $column, $type),
            " with ",
            $with
        )
    };
}

mod iam {
    use sqlx::{postgres::PgRow, FromRow, Row};

    use crate::app::resource::iam::UserResponse;
    use crate::domain::entity::{
        iam::{User, UserState},
        EntityData,
    };

    impl<'r> FromRow<'r, PgRow> for UserState {
        fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
            Ok(Self {
                email: row.try_get("email")?,
                bio: row.try_get("bio")?,
                image_url: row.try_get::<Option<&str>, &str>("image_url")?.map(|s| {
                    s.parse().expect(column_decode_error!(
                        "user",
                        "image_url",
                        "TEXT",
                        "valid url"
                    ))
                }),
                password_hash: row.try_get::<&str, &str>("password_hash")?.parse().expect(
                    column_decode_error!(
                        "user",
                        "password_hash",
                        "TEXT",
                        "valid password_hash encoding"
                    ),
                ),
                username: row.try_get("username")?,
            })
        }
    }

    impl<'r> FromRow<'r, PgRow> for User {
        fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
            Ok(Self {
                data: EntityData::from_row(row)?,
                state: UserState::from_row(row)?,
            })
        }
    }

    impl From<User> for UserResponse {
        fn from(user: User) -> Self {
            let (ent, state) = user.unmount_state();
            Self {
                id: ent.id,
                created: ent.created,
                updated: ent.updated,
                version: ent.version,
                username: state.username,
                email: state.email,
                bio: state.bio,
                image_url: state.image_url,
            }
        }
    }
}
