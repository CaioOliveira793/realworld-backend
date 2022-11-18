pub mod database;

pub mod handler {
    use super::database::{extract_client, repository};
    use crate::{
        app::resource::iam::{CreateUserDto, UserResponse},
        domain::entity::iam::User,
        error::http::BadRequest,
    };

    use async_trait::async_trait;
    use deadpool_postgres::Pool;
    use salvo::{
        http::StatusCode, prelude::StatusError, writer::Json, Depot, FlowCtrl, Handler, Request,
        Response,
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
            let result: Result<CreateUserDto, _> = req.parse_body().await.map_err(BadRequest::from);
            let dto = map_res_err!(result, res);

            let user = User::from(dto);

            let result = extract_client(&self.db_pool).await;
            let client = map_res_err!(result, res);

            let result =
                repository::usernames_exists(&client, [user.username().clone()].iter()).await;
            let usernames = map_res_err!(result, res);
            if !usernames.is_empty() {
                res.set_status_error(StatusError::bad_request());
                return;
            }

            let result = repository::insert_user(&client, [user.clone()].iter()).await;
            map_res_err!(result, res);

            res.render(Json(UserResponse::from(user)));
            res.set_status_code(StatusCode::CREATED);
        }
    }
}

pub mod query {}

pub mod validation {}

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
        Router::with_path("user").post(CreateUserHandler::new(db_pool))
    }
}
