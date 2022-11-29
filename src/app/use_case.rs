pub mod iam {
    use sqlx::PgPool;

    use crate::{
        app::resource::iam::{CreateUserDto, UserResponse},
        base::ResourceID,
        domain::{entity::iam::User, service::PasswordHashService},
        error::{
            app::ApplicationError,
            resource::{ValidationError, ValidationErrorKind, ValidationFieldError},
        },
        infra::database::repository,
    };

    mod validation {
        use super::*;

        pub async fn create_user<'dto>(
            pool: &PgPool,
            dto: &CreateUserDto<'dto>,
        ) -> Result<(), ApplicationError<CreateUserDto<'dto>>> {
            let mut errors = Vec::new();

            let emails = repository::email_exists(pool, [&dto.email.into()]).await?;
            if !emails.is_empty() {
                errors.push(ValidationFieldError::new(
                    "base::email".into(),
                    dto.email.into(),
                    "/email".into(),
                    vec![ValidationErrorKind::AlreadyExists],
                ));
            }

            let usernames = repository::username_exists(pool, [&dto.username.into()]).await?;
            if !usernames.is_empty() {
                errors.push(ValidationFieldError::new(
                    "base::username".into(),
                    dto.username.into(),
                    "/username".into(),
                    vec![ValidationErrorKind::AlreadyExists],
                ));
            }

            if !errors.is_empty() {
                return Err(ValidationError::from_resource(dto.clone(), errors).into());
            }

            Ok(())
        }
    }

    pub async fn create_user<'dto, HS: PasswordHashService>(
        pool: &PgPool,
        hash_service: &HS,
        dto: CreateUserDto<'dto>,
    ) -> Result<UserResponse, ApplicationError<CreateUserDto<'dto>>> {
        validation::create_user(pool, &dto).await?;

        let password_hash = hash_service.hash_password(dto.password).map_err(|_| {
            ValidationError::from_resource(
                dto.clone(),
                vec![ValidationFieldError::new(
                    "base::password".into(),
                    dto.password.into(),
                    "/password".into(),
                    vec![ValidationErrorKind::Invalid],
                )],
            )
        })?;
        let user = User::new(dto.email.into(), dto.username.into(), password_hash);

        repository::insert_user(pool, [&user]).await?;

        Ok(user.into())
    }
}
