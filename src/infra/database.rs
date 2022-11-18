use deadpool_postgres::{Client, Config, ManagerConfig, Pool, RecyclingMethod, Runtime, SslMode};
use tokio_postgres::NoTls;
use tokio_postgres_rustls::MakeRustlsConnect;

use crate::{config, error::storage::DatabaseError};

fn pool_config() -> Config {
    let config = config::env_var::get().clone();

    let mut cfg = Config::new();
    cfg.host = Some(config.database_host);
    cfg.dbname = Some(config.database_name);
    cfg.port = Some(config.database_port);
    cfg.user = Some(config.database_user);
    cfg.password = Some(config.database_password);
    cfg.manager = Some(ManagerConfig {
        recycling_method: RecyclingMethod::Fast,
    });
    cfg.application_name = Some("Conduit".into());
    cfg.ssl_mode = Some(SslMode::Prefer);
    cfg
}

// TODO: use database connection with tls
#[allow(dead_code)]
fn tls_config() -> MakeRustlsConnect {
    let mut root_store = rustls::RootCertStore::empty();
    root_store.add_server_trust_anchors(webpki_roots::TLS_SERVER_ROOTS.0.iter().map(|ta| {
        rustls::OwnedTrustAnchor::from_subject_spki_name_constraints(
            ta.subject,
            ta.spki,
            ta.name_constraints,
        )
    }));
    let tls_config = rustls::ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(root_store)
        .with_no_client_auth();
    MakeRustlsConnect::new(tls_config)
}

pub async fn create_pool() -> Pool {
    async fn connect(pool: &Pool) {
        let c = pool.get().await.expect("could not connect to database");
        drop(c);
    }
    let cfg = pool_config();
    let pool = cfg
        .create_pool(Some(Runtime::Tokio1), NoTls)
        .expect("should create a connection pool");
    #[cfg(feature = "connect_db_on_start")]
    connect(&pool).await;
    pool
}

pub async fn extract_client(pool: &Pool) -> Result<Client, DatabaseError> {
    // TODO: handle timeout with retry
    Ok(pool.get().await?)
}

pub mod sql {
    use std::ops::Range;

    use postgres_types::ToSql;

    pub type Param<'a> = &'a (dyn ToSql + Sync);

    pub fn list_params(query: &mut String, range: Range<usize>) {
        query.push('(');
        for idx in range {
            query.push_str(format!("${}", idx).as_str());
        }
        query.push(')');
    }

    pub fn ref_tosql<T: ToSql + Sync>(v: &T) -> &(dyn ToSql + Sync) {
        v as &(dyn ToSql + Sync)
    }

    pub const USER_COLUMN_COUNT: usize = 6;
}

pub mod repository {
    use std::collections::HashSet;

    use deadpool_postgres::Client;

    use super::sql;
    use crate::{domain::entity::iam::User, error::storage::DatabaseError};

    pub async fn insert_user<'u, I>(client: &Client, users: I) -> Result<(), DatabaseError>
    where
        I: IntoIterator<Item = &'u User>,
    {
        let mut params: Vec<sql::Param> = Vec::new();
        let mut rows = 0;

        for user in users {
            rows += 1;
            // params.push(&user.ident());
            params.push(user.username());
            params.push(user.email());
            params.push(user.password_hash());
            params.push(user.bio());
            // params.push(user.image_url().map(|url| url.as_str()));
        }

        let mut query = String::from(concat!(
            "INSERT INTO user (id, username, email, password_hash, bio, image_url) ",
            "VALUES ",
        ));

        for idx in 0..rows {
            sql::list_params(
                &mut query,
                sql::USER_COLUMN_COUNT * idx + 1..sql::USER_COLUMN_COUNT * (idx + 1) + 1,
            );
        }

        client.query(&query, &params).await?;
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn find_user<I>(
        client: &Client,
        username: String,
    ) -> Result<Option<User>, DatabaseError>
    where
        I: IntoIterator<Item = User>,
    {
        let query = concat!(
            "SELECT user.id, user.username, user.email, user.password_hash, ",
            "user.bio, user.image_url FROM user WHERE user.username = $1",
        );

        if let Some(row) = client.query_opt(query, &[&username]).await? {
            return Ok(Some(User::from(&row)));
        }

        Ok(None)
    }

    pub async fn usernames_exists<'u, I>(
        client: &Client,
        usernames: I,
    ) -> Result<HashSet<String>, DatabaseError>
    where
        I: Iterator<Item = &'u String>,
    {
        let unames: Vec<sql::Param> = usernames.into_iter().map(sql::ref_tosql).collect();

        let mut query = String::from("SELECT user.username FROM user WHERE user.username IN ");
        sql::list_params(&mut query, 1..unames.len() + 1);

        let rows = client.query(&query, &unames).await?;

        Ok(rows.into_iter().map(|row| row.get("username")).collect())
    }
}
