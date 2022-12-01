use std::sync::Arc;

use async_trait::async_trait;
use salvo::{http::StatusCode, writer::Json, Depot, FlowCtrl, Handler, Request, Response};
use sqlx::PgPool;

use crate::app::{
    resource::iam::{CreateUserDto, UserCredential},
    use_case,
};
use crate::error::http::BadRequest;
use crate::infra::service::{Argon2HashService, JWTEncryptionService};

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

pub struct CreateUserController {
    pool: PgPool,
    hash_service: Arc<Argon2HashService>,
}

impl CreateUserController {
    pub fn new(pool: PgPool, hash_service: Arc<Argon2HashService>) -> Self {
        Self { pool, hash_service }
    }
}

#[async_trait]
impl Handler for CreateUserController {
    async fn handle(&self, req: &mut Request, _: &mut Depot, res: &mut Response, _: &mut FlowCtrl) {
        let result: Result<CreateUserDto, _> = req.parse_body().await.map_err(BadRequest::from);
        let dto = map_res_err!(result, res);

        let result = use_case::iam::create_user(&self.pool, self.hash_service.as_ref(), dto).await;
        let user = map_res_err!(result, res);

        res.render(Json(user));
        res.set_status_code(StatusCode::CREATED);
    }
}

pub struct AuthenticateUserController {
    pool: PgPool,
    hash_service: Arc<Argon2HashService>,
    token_service: Arc<JWTEncryptionService>,
}

impl AuthenticateUserController {
    pub fn new(
        pool: PgPool,
        hash_service: Arc<Argon2HashService>,
        token_service: Arc<JWTEncryptionService>,
    ) -> Self {
        Self {
            pool,
            hash_service,
            token_service,
        }
    }
}

#[async_trait]
impl Handler for AuthenticateUserController {
    async fn handle(&self, req: &mut Request, _: &mut Depot, res: &mut Response, _: &mut FlowCtrl) {
        let result: Result<UserCredential, _> = req.parse_body().await.map_err(BadRequest::from);
        let credential = map_res_err!(result, res);

        let result = use_case::iam::authenticate_user(
            &self.pool,
            self.hash_service.as_ref(),
            self.token_service.as_ref(),
            credential,
        )
        .await;
        let auth_response = map_res_err!(result, res);

        res.render(Json(auth_response));
        res.set_status_code(StatusCode::OK);
    }
}
