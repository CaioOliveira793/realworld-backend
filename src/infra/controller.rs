use std::sync::Arc;

use async_trait::async_trait;
use salvo::{http::StatusCode, writer::Json, Depot, FlowCtrl, Handler, Request, Response};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::app::ApplicationError;
use crate::error::http::BadRequest;
use crate::infra::service::{Argon2HashService, JWTEncryptionService};
use crate::{
    app::{
        resource::iam::{CreateUser, UpdateUser, UserCredential},
        use_case,
    },
    error::security::AuthenticationError,
};

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
        let id = extract_id(req);
        let result: Result<CreateUser, _> = req.parse_body().await.map_err(BadRequest::from);
        let dto = map_res_err!(result, res);

        let result =
            use_case::iam::create_user(&self.pool, self.hash_service.as_ref(), id, dto).await;
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

pub struct UpdateUserController {
    pool: PgPool,
    token_service: Arc<JWTEncryptionService>,
}

impl UpdateUserController {
    pub fn new(pool: PgPool, token_service: Arc<JWTEncryptionService>) -> Self {
        Self {
            pool,
            token_service,
        }
    }
}

/// Extract a authorization token from a request.
///
/// Token must be formated in the Bearer authentication scheme
/// described in [RFC 7617](https://datatracker.ietf.org/doc/html/rfc7617)
fn extract_token<'req>(req: &'req Request) -> Result<&'req str, AuthenticationError> {
    let scheme: Option<&str> = req.header("authorization");
    scheme
        .ok_or(AuthenticationError::TokenNotPresent)?
        .strip_prefix("Bearer ")
        .ok_or(AuthenticationError::MalformattedToken)
}

/// Extract a uuid from a request id param
///
/// # Panic
///
/// Panics if a id param is not present or the content is not a valid uuid
fn extract_id<'req>(req: &'req Request) -> Uuid {
    req.params()
        .get("id")
        .expect("Expect to route only with valid uuid")
        .parse()
        .expect("Expect id param as a valid uuid")
}

#[async_trait]
impl Handler for UpdateUserController {
    async fn handle(&self, req: &mut Request, _: &mut Depot, res: &mut Response, _: &mut FlowCtrl) {
        let result: Result<UpdateUser, _> = req.parse_body().await.map_err(BadRequest::from);
        let dto = map_res_err!(result, res);

        let id = extract_id(req);
        let result = extract_token(req).map_err(ApplicationError::<()>::from);
        let tk = map_res_err!(result, res);

        let result =
            use_case::iam::update_user(&self.pool, self.token_service.as_ref(), tk, id, dto).await;
        let resource = map_res_err!(result, res);

        res.render(Json(resource));
        res.set_status_code(StatusCode::OK);
    }
}
