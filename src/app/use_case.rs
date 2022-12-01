pub mod iam {
    use std::time::Duration;

    use sqlx::PgPool;

    use crate::{
        app::resource::iam::{
            AuthenticateUserResponse, CreateUserDto, UserCredential, UserResponse,
        },
        domain::{
            datatype::security::{Token, TokenPayload, TokenSubject},
            entity::{iam::User, Entity},
            service::{PasswordHashService, TokenEncryptionService},
        },
        error::{
            app::ApplicationError,
            resource::{ValidationError, ValidationErrorKind, ValidationFieldError},
            security::AuthenticationError,
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
                    "base::email",
                    dto.email.into(),
                    "/email".into(),
                    vec![ValidationErrorKind::AlreadyExists],
                ));
            }

            let usernames = repository::username_exists(pool, [&dto.username.into()]).await?;
            if !usernames.is_empty() {
                errors.push(ValidationFieldError::new(
                    "base::username",
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
                    "base::password",
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

    const AUTHENTICATION_TOKEN_EXPIRATION: Duration = Duration::from_secs(60 * 60 * 8);

    pub async fn authenticate_user<'dto, HS, TS>(
        pool: &PgPool,
        hash_service: &HS,
        token_service: &TS,
        credential: UserCredential<'dto>,
    ) -> Result<AuthenticateUserResponse, ApplicationError<UserCredential<'dto>>>
    where
        HS: PasswordHashService,
        TS: TokenEncryptionService,
    {
        let user = repository::find_user_by_email(pool, credential.email.into())
            .await?
            .ok_or_else(|| {
                ValidationError::from_resource(
                    credential.clone(),
                    vec![ValidationFieldError::new(
                        "base::email",
                        credential.email.into(),
                        "/email".into(),
                        vec![ValidationErrorKind::NotFound],
                    )],
                )
            })?;

        if hash_service
            .verify_password(credential.password, user.password_hash())
            .is_err()
        {
            return Err(AuthenticationError::InvalidCredential.into());
        }

        let payload = TokenPayload::new(
            AUTHENTICATION_TOKEN_EXPIRATION,
            TokenSubject::User(user.ident()),
            (),
        );
        let token =
            Token::new(payload, token_service).expect("Expect to sign a user authentication token");

        Ok(AuthenticateUserResponse {
            user: user.into(),
            token: token.into(),
        })
    }
}
