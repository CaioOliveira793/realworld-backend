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

pub mod sql {
    use sqlx::{Database, Encode, QueryBuilder, Type};

    pub fn push_list<'args, I, T, DB>(qb: &mut QueryBuilder<'args, DB>, list: I)
    where
        I: IntoIterator<Item = T>,
        T: 'args + Encode<'args, DB> + Send + Type<DB>,
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

    use super::sql;
    use crate::{
        domain::entity::{iam::User, Entity},
        error::persistence::PersistenceError,
    };

    #[instrument(target = "database::iam::user", skip(pool))]
    pub async fn insert_user<'u, I>(pool: &PgPool, users: I) -> Result<(), PersistenceError>
    where
        I: IntoIterator<Item = &'u User> + std::fmt::Debug,
    {
        let mut qb = QueryBuilder::new(
            "INSERT INTO iam.user (id, created, updated, version, username, email, password_hash, bio, image_url) "
        );

        qb.push_values(users.into_iter(), |mut qb, user| {
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

        qb.build().execute(pool).await?;

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

    macro_rules! make_fn_value_exists {
        ($fn_name:ident, $table:literal, $column:literal, $type:ty) => {
            #[instrument(skip(pool))]
            pub async fn $fn_name<'e, I>(
                pool: &PgPool,
                values: I,
            ) -> Result<HashSet<$type>, PersistenceError>
            where
                I: IntoIterator<Item = &'e $type> + std::fmt::Debug,
            {
                let mut qb = QueryBuilder::new(concat!(
                    "SELECT ", $column, " FROM ", $table, " WHERE ", $column, " IN "
                ));
                sql::push_list(&mut qb, values);

                let mut rows = qb.build().fetch(pool);

                let mut set = HashSet::new();
                while let Some(row) = rows.try_next().await? {
                    set.insert(row.get(0));
                }

                Ok(set)
            }
        };
    }

    make_fn_value_exists!(email_exists, "iam.user", "email", String);
    make_fn_value_exists!(username_exists, "iam.user", "username", String);
}
