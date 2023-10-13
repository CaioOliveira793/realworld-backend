pub mod controller;
pub mod database;
pub mod service;

pub mod query {}

pub mod router {
    use std::sync::Arc;

    use salvo::{logging::Logger, routing::PathFilter, Router};
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
        PathFilter::register_wisp_regex(
            "uuid",
            regex::Regex::new(
                "^[0-9A-F]{8}-[0-9A-F]{4}-4[0-9A-F]{3}-[89AB][0-9A-F]{3}-[0-9A-F]{12}$",
            )
            .expect("Expect a valid uuid v4 regex"),
        );

        Router::new()
            .push(
                Router::with_path("api")
                    .push(
                        Router::with_path("user/<id:uuid>")
                            .post(CreateUserController::new(
                                pool.clone(),
                                hash_service.clone(),
                            ))
                            .put(UpdateUserController::new(
                                pool.clone(),
                                token_service.clone(),
                            )),
                    )
                    .push(Router::with_path("auth/<id:uuid>").post(
                        AuthenticateUserController::new(pool.clone(), hash_service, token_service),
                    )),
            )
            .hoop(Logger)
    }
}
