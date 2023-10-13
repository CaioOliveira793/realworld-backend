pub mod connection {
    use std::time::Duration;

    use crate::config::env_var;

    pub async fn create_sqlx_pool() -> sqlx::PgPool {
        let dburl = env_var::get().database_url.clone();
        sqlx::postgres::PgPoolOptions::new()
            .min_connections(1)
            .max_connections(5)
            .acquire_timeout(Duration::from_millis(1000))
            .idle_timeout(Duration::from_millis(1000 * 30))
            .max_lifetime(Duration::from_millis(1000 * 10))
            .connect(&dburl)
            .await
            .expect("Expect to create a database pool with a open connection")
    }
}

mod sql {
    use sqlx::{Database, Encode, QueryBuilder, Type};

    pub fn push_list<'args, I, T, DB>(qb: &mut QueryBuilder<'args, DB>, list: I)
    where
        I: IntoIterator<Item = T>,
        T: 'args + Encode<'args, DB> + Type<DB> + Send,
        DB: Database,
    {
        qb.push("(");
        let mut sep = qb.separated(", ");
        for item in list {
            sep.push_bind(item);
        }
        sep.push_unseparated(")");
    }
}

pub mod repository {
    use std::collections::HashSet;

    use futures::TryStreamExt;
    use sqlx::{FromRow, PgPool, QueryBuilder, Row};
    use tracing::instrument;
    use uuid::Uuid;

    use super::sql;
    use crate::{
        app::resource::iam::UserResponse,
        domain::entity::{iam::User, Entity},
        error::{
            persistence::{MutationError, PersistenceError},
            resource::ConflictError,
        },
    };

    #[instrument(target = "database::iam::user", skip(pool))]
    pub async fn insert_users<'u, I>(pool: &PgPool, users: I) -> Result<(), MutationError>
    where
        I: IntoIterator<Item = &'u User> + std::fmt::Debug,
    {
        let mut qb = QueryBuilder::new(
            "INSERT INTO iam.user (id, created, updated, version, username, email, password_hash, bio, image_url) "
        );
        qb.push_values(users, |mut qb, user| {
            qb.push_bind(user.ident());
            qb.push_bind(user.created());
            qb.push_bind(user.updated());
            qb.push_bind(user.version() as i64);
            qb.push_bind(user.username());
            qb.push_bind(user.email());
            qb.push_bind(user.password_hash().to_string());
            qb.push_bind(user.bio());
            qb.push_bind(user.image_url().clone().map(|url| url.to_string()));
        });
        qb.push(" ON CONFLICT (id) DO NOTHING");

        let afected = qb
            .build()
            .execute(pool)
            .await
            .map_err(PersistenceError::from)?;

        if afected.rows_affected() == 0 {
            return Err(ConflictError::from_resource::<UserResponse>(None).into());
        }

        Ok(())
    }

    #[instrument(target = "database::iam::user", skip(pool))]
    pub async fn update_user<'u>(pool: &PgPool, user: &'u User) -> Result<(), MutationError> {
        let afected = sqlx::query(concat!(
            "UPDATE TABLE iam.user SET updated = $1, version = $2, username = $3, ",
            "email = $4, password_hash = $5, bio = $6, image_url = $7 ",
            "WHERE id = $8 AND version = $9"
        ))
        .bind(user.updated())
        .bind(user.version() as i64)
        .bind(user.username())
        .bind(user.email())
        .bind(user.password_hash().to_string())
        .bind(user.bio())
        .bind(user.image_url().clone().map(|url| url.to_string()))
        .bind(user.ident())
        .bind(user.version() as i64 - 1)
        .execute(pool)
        .await
        .map_err(PersistenceError::from)?;

        if afected.rows_affected() == 0 {
            return Err(ConflictError::from_resource::<UserResponse>(Some(user.ident())).into());
        }

        Ok(())
    }

    #[instrument(target = "database::iam::user", skip(pool))]
    pub async fn find_user_by_email(
        pool: &PgPool,
        email: String,
    ) -> Result<Option<User>, PersistenceError> {
        let row = sqlx::query(concat!(
            "SELECT id, created, updated, version, username, email, password_hash, ",
            "bio, image_url FROM iam.user WHERE email = $1",
        ))
        .bind(email)
        .fetch_optional(pool)
        .await?;

        if let Some(row) = row {
            return Ok(Some(User::from_row(&row)?));
        }

        Ok(None)
    }

    #[instrument(target = "database::iam::user", skip(pool))]
    pub async fn find_user_by_id(
        pool: &PgPool,
        id: Uuid,
    ) -> Result<Option<User>, PersistenceError> {
        let row = sqlx::query(concat!(
            "SELECT id, created, updated, version, username, email, password_hash, ",
            "bio, image_url FROM iam.user WHERE id = $1",
        ))
        .bind(id)
        .fetch_optional(pool)
        .await?;

        if let Some(row) = row {
            return Ok(Some(User::from_row(&row)?));
        }

        Ok(None)
    }

    macro_rules! query_column_list {
        ($pool:ident, $values:ident, $query:literal) => {
            async {
                let mut qb = QueryBuilder::new($query);
                sql::push_list(&mut qb, $values);

                let mut rows = qb.build().fetch($pool);

                let mut set = HashSet::new();
                while let Some(row) = rows.try_next().await? {
                    set.insert(row.get(0));
                }

                Ok(set)
            }
        };
    }

    #[instrument(skip(pool))]
    pub async fn email_exists<'a, I>(
        pool: &PgPool,
        values: I,
    ) -> Result<HashSet<String>, PersistenceError>
    where
        I: IntoIterator<Item = &'a String> + std::fmt::Debug,
    {
        query_column_list!(pool, values, "SELECT email FROM iam.user WHERE email IN ").await
    }

    #[instrument(skip(pool))]
    pub async fn username_exists<'a, I>(
        pool: &PgPool,
        values: I,
    ) -> Result<HashSet<String>, PersistenceError>
    where
        I: IntoIterator<Item = &'a String> + std::fmt::Debug,
    {
        query_column_list!(
            pool,
            values,
            "SELECT username FROM iam.user WHERE username IN "
        )
        .await
    }
}
