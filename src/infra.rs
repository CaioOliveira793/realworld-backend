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

    macro_rules! get_env {
        ($env:literal) => {
            std::env::var($env).expect(concat!("Missing env var ", $env))
        };
    }

    fn pool_config() -> Config {
        let port: u16 = get_env!("DATABASE_PORT")
            .parse()
            .expect("Invalid DATABASE_PORT");

        let mut cfg = Config::new();
        cfg.host = Some(get_env!("DATABASE_HOST"));
        cfg.dbname = Some(get_env!("DATABASE_NAME"));
        cfg.port = Some(port);
        cfg.user = Some(get_env!("DATABASE_USER"));
        cfg.password = Some(get_env!("DATABASE_PASSWORD"));
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
        use sea_query::{Iden, InsertStatement, Query};

        use crate::domain::entity::User;

        #[derive(Iden)]
        #[iden = "user"]
        enum UserTable {
            Table,
            #[iden = "id"]
            Id,
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

        pub fn insert_user<I>(users: I) -> InsertStatement
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
                    user.email.into(),
                    user.password.into(),
                    user.username.into(),
                    user.bio.into(),
                    user.image.into(),
                ]);
            }

            sttm
        }
    }

    pub mod repository {
        use deadpool_postgres::Client;
        use sea_query::{PostgresDriver, PostgresQueryBuilder};

        use super::sql;
        use crate::{domain::entity::User, infra::error::RepositoryError};

        pub async fn insert_user<I>(client: Client, users: I) -> Result<(), RepositoryError>
        where
            I: IntoIterator<Item = User>,
        {
            let sttm = sql::insert_user(users).build(PostgresQueryBuilder);
            client.query(&sttm.0, &sttm.1.as_params()).await?;
            Ok(())
        }
    }
}

pub mod error {
    use derive_more::{Display, Error};
    use salvo::{prelude::StatusError, Piece, Response};
    use tokio_postgres::error::DbError;

    #[derive(Debug, Display)]
    pub struct UnknownError(Box<dyn std::error::Error + Send + Sync + 'static>);

    #[derive(Debug, Display, Error)]
    pub enum RepositoryError {
        #[display(fmt = "database error: {_0}")]
        Db(DbError),
        #[display(fmt = "unknown database error: {_0}")]
        Unknown(UnknownError),
    }

    impl std::error::Error for UnknownError {}

    impl From<tokio_postgres::Error> for UnknownError {
        fn from(err: tokio_postgres::Error) -> Self {
            UnknownError(err.into())
        }
    }

    impl Piece for UnknownError {
        fn render(self, res: &mut Response) {
            res.set_status_error(StatusError::internal_server_error());
            // TODO: add body describing the error
        }
    }

    impl From<tokio_postgres::Error> for RepositoryError {
        fn from(err: tokio_postgres::Error) -> Self {
            if let Some(db_err) = err.as_db_error() {
                return RepositoryError::Db(db_err.clone());
            }

            RepositoryError::Unknown(err.into())
        }
    }

    impl Piece for RepositoryError {
        fn render(self, res: &mut Response) {
            res.set_status_error(StatusError::service_unavailable());
            // TODO: add body describing the error
        }
    }
}

pub mod handler {
    use super::{
        database::{extract_client, repository},
        resource::*,
    };
    use crate::{
        app::resource::{CreateUserDto, UserResponse},
        domain::entity::User,
    };

    use async_trait::async_trait;
    use deadpool_postgres::Pool;
    use salvo::{writer::Json, Depot, FlowCtrl, Handler, Piece, Request, Response};

    fn handle_piece_err<V, E>(result: Result<V, E>, response: &mut Response) -> Option<V>
    where
        E: Piece,
    {
        match result {
            Err(err) => {
                response.render(err);
                None
            }
            Ok(res) => Some(res),
        }
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
            let req_data: UserResource<CreateUserDto> =
                req.parse_body().await.expect("invalid body");

            let user = User::from(req_data.user);

            let client = extract_client(&self.db_pool).await;
            handle_piece_err(repository::insert_user(client, [user.clone()]).await, res);

            let res_data = UserResource::<UserResponse> { user: user.into() };
            res.render(Json(res_data));
        }
    }
}

pub mod router {
    use deadpool_postgres::Pool;
    use salvo::Router;

    use super::handler::*;

    pub fn app(db_pool: Pool) -> Router {
        Router::with_path("api").push(user(db_pool))
    }

    pub fn user(db_pool: Pool) -> Router {
        Router::with_path("users").post(CreateUserHandler::new(db_pool))
    }
}
