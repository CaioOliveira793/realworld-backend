pub mod controller;
pub mod database;
pub mod service;

pub mod query {}

pub mod router {
    use std::sync::Arc;

    use salvo::{logging::Logger, Router};
    use sqlx::PgPool;

    use super::{
        controller::*,
        service::{Argon2HashService, JWTEncryptionService},
    };

    pub fn app(
        pool: &PgPool,
        hash_service: Arc<Argon2HashService>,
        token_service: Arc<JWTEncryptionService>,
    ) -> Router {
        Router::new()
            .push(
                Router::with_path("api")
                    .push(Router::with_path("user").post(CreateUserController::new(
                        pool.clone(),
                        hash_service.clone(),
                    )))
                    .push(
                        Router::with_path("auth").post(AuthenticateUserController::new(
                            pool.clone(),
                            hash_service,
                            token_service,
                        )),
                    ),
            )
            .hoop(Logger)
    }
}
