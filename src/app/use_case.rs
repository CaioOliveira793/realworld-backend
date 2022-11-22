pub mod iam {
    use sqlx::PgPool;

    use crate::{
        app::resource::iam::{CreateUserDto, UserResponse},
        domain::entity::iam::User,
        error::{
            app::OperationError,
            resource::{ValidationError, ValidationErrorKind, ValidationFieldError},
        },
        infra::database::repository,
    };

    pub async fn create_user<'dto>(
        pool: &PgPool,
        dto: CreateUserDto<'dto>,
    ) -> Result<UserResponse, OperationError<CreateUserDto<'dto>>> {
        let user = User::from(dto.clone());

        let usernames = repository::usernames_exists(&pool, [user.username()]).await?;

        if !usernames.is_empty() {
            return Err(OperationError::domain(
                "iam",
                ValidationError {
                    resource: dto,
                    resource_type: "iam::user".into(),
                    fields: vec![ValidationFieldError {
                        path: "/username".into(),
                        type_id: "username".into(),
                        value: user.username().clone(),
                        kinds: vec![ValidationErrorKind::AlreadyExists],
                    }],
                }
                .into(),
            ));
        }

        repository::insert_user(&pool, [&user]).await?;

        Ok(user.into())
    }
}
