pub mod error;

mod resource {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Deserialize, Serialize)]
    pub struct UserResource<T> {
        pub user: T,
    }
}

pub mod database {
    use deadpool_postgres::{
        Client, Config, ManagerConfig, Pool, RecyclingMethod, Runtime, SslMode,
    };
    use tokio_postgres::NoTls;
    use tokio_postgres_rustls::MakeRustlsConnect;

    use crate::config;

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
        let pool = cfg.create_pool(Some(Runtime::Tokio1), NoTls).unwrap();
        #[cfg(feature = "connect_db_on_start")]
        connect(&pool).await;
        pool
    }

    pub async fn extract_client(pool: &Pool) -> Client {
        // TODO: handle timeout with retry
        pool.get().await.unwrap()
    }

    pub mod sql {
        use sea_query::{Alias, Expr, Iden, InsertStatement, Query, SelectStatement};

        use crate::domain::entity::User;

        #[derive(Iden)]
        #[iden = "user"]
        enum UserTable {
            Table,
            #[iden = "email"]
            Email,
            #[iden = "password"]
            Password,
            #[iden = "username"]
            Username,
            #[iden = "bio"]
            Bio,
            #[iden = "image_url"]
            Image,
        }

        pub fn insert_users<I>(users: I) -> InsertStatement
        where
            I: IntoIterator<Item = User>,
        {
            let mut sttm = Query::insert();
            sttm.into_table(UserTable::Table);
            sttm.columns([
                UserTable::Email,
                UserTable::Password,
                UserTable::Username,
                UserTable::Bio,
                UserTable::Image,
            ]);

            for user in users {
                sttm.values_panic([
                    user.email().into(),
                    user.password_hash().into(),
                    user.username().into(),
                    user.bio().into(),
                    user.image_url().into(),
                ]);
            }

            sttm
        }

        pub fn select_users<I>(username: I) -> SelectStatement
        where
            I: IntoIterator<Item = String>,
        {
            let mut sttm = Query::select();
            sttm.expr_as(Expr::col(UserTable::Username), Alias::new("username"));
            sttm.expr_as(Expr::col(UserTable::Email), Alias::new("email"));
            sttm.expr_as(Expr::col(UserTable::Password), Alias::new("password"));
            sttm.expr_as(Expr::col(UserTable::Bio), Alias::new("bio"));
            sttm.expr_as(Expr::col(UserTable::Image), Alias::new("image"));
            sttm.from(UserTable::Table);
            sttm.and_where(Expr::col(UserTable::Username).is_in(username));
            sttm
        }

        pub fn select_usernames<I>(username: I) -> SelectStatement
        where
            I: IntoIterator<Item = String>,
        {
            let mut sttm = Query::select();
            sttm.expr_as(Expr::col(UserTable::Username), Alias::new("username"));
            sttm.from(UserTable::Table);
            sttm.and_where(Expr::col(UserTable::Username).is_in(username));
            sttm
        }
    }

    pub mod repository {
        use std::collections::HashSet;

        use deadpool_postgres::Client;
        use sea_query::{PostgresDriver, PostgresQueryBuilder};

        use super::sql;
        use crate::{domain::entity::User, infra::error::storage::RepositoryError};

        pub async fn insert_user<I>(client: &Client, users: I) -> Result<(), RepositoryError>
        where
            I: IntoIterator<Item = User>,
        {
            let sttm = sql::insert_users(users).build(PostgresQueryBuilder);
            client.query(&sttm.0, &sttm.1.as_params()).await?;
            Ok(())
        }

        pub async fn find_user<I>(
            client: &Client,
            username: String,
        ) -> Result<Option<User>, RepositoryError>
        where
            I: IntoIterator<Item = User>,
        {
            let sttm = sql::select_users([username]).build(PostgresQueryBuilder);
            let row = client.query_opt(&sttm.0, &sttm.1.as_params()).await?;

            if let Some(row) = row {
                return Ok(Some(User::from(&row)));
            }

            Ok(None)
        }

        pub async fn usernames_exists<I>(
            client: &Client,
            usernames: I,
        ) -> Result<HashSet<String>, RepositoryError>
        where
            I: IntoIterator<Item = String>,
        {
            let sttm = sql::select_usernames(usernames).build(PostgresQueryBuilder);
            let rows = client.query(&sttm.0, &sttm.1.as_params()).await?;
            let found_usernames: HashSet<String> =
                rows.into_iter().map(|row| row.get("username")).collect();
            Ok(found_usernames)
        }
    }
}

pub mod handler {
    use super::{
        database::{extract_client, repository},
        resource::*,
    };
    use crate::{
        app::resource::iam::{CreateUserDto, UserResponse},
        domain::entity::User,
        infra::error::http::BadRequest,
    };

    use async_trait::async_trait;
    use deadpool_postgres::Pool;
    use reqwest::StatusCode;
    use salvo::{prelude::StatusError, writer::Json, Depot, FlowCtrl, Handler, Request, Response};

    macro_rules! map_res_err {
        ($result:ident, $response:ident) => {
            match $result {
                Err(err) => {
                    $response.render(err);
                    return;
                }
                Ok(ok) => ok,
            }
        };
    }

    pub struct CreateUserHandler {
        db_pool: Pool,
    }

    impl CreateUserHandler {
        pub fn new(db_pool: Pool) -> Self {
            Self { db_pool }
        }
    }

    #[async_trait]
    impl Handler for CreateUserHandler {
        async fn handle(
            &self,
            req: &mut Request,
            _: &mut Depot,
            res: &mut Response,
            _: &mut FlowCtrl,
        ) {
            let result: Result<UserResource<CreateUserDto>, _> =
                req.parse_body().await.map_err(BadRequest::from);
            let req_data = map_res_err!(result, res);

            let user = User::from(req_data.user);

            let client = extract_client(&self.db_pool).await;

            let result = repository::usernames_exists(&client, [user.username().clone()]).await;
            let usernames = map_res_err!(result, res);
            if !usernames.is_empty() {
                res.set_status_error(StatusError::bad_request());
            }

            let result = repository::insert_user(&client, [user.clone()]).await;
            map_res_err!(result, res);

            let res_data = UserResource::<UserResponse> { user: user.into() };
            res.render(Json(res_data));
            res.set_status_code(StatusCode::CREATED);
        }
    }
}

pub mod router {
    use deadpool_postgres::Pool;
    use salvo::{logging::Logger, Router};

    use super::handler::*;

    pub fn app(db_pool: Pool) -> Router {
        Router::new()
            .hoop(Logger)
            .push(Router::with_path("api").push(user(db_pool)))
    }

    pub fn user(db_pool: Pool) -> Router {
        Router::with_path("users").post(CreateUserHandler::new(db_pool))
    }
}
