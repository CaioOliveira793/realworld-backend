use derive_more::Display;
use salvo::{prelude::StatusError, writer::Json, Piece, Response};

use self::http::ErrorResponse;

pub type BoxedError = Box<dyn std::error::Error + Send + Sync + 'static>;

#[derive(Debug, Display)]
pub struct UnknownError(BoxedError);

impl std::error::Error for UnknownError {}

impl UnknownError {
    pub fn new(err: BoxedError) -> Self {
        Self(err)
    }

    pub fn inner(self) -> BoxedError {
        self.0
    }

    pub fn ref_inner(&self) -> &BoxedError {
        &self.0
    }

    pub fn ref_mut_inner(&mut self) -> &mut BoxedError {
        &mut self.0
    }
}

impl From<BoxedError> for UnknownError {
    fn from(err: BoxedError) -> Self {
        Self::new(err)
    }
}

#[derive(Debug, Display)]
struct StrError(pub Box<str>);

impl std::error::Error for StrError {}

impl<'a> From<&'a str> for UnknownError {
    fn from(err: &'a str) -> Self {
        UnknownError(Box::new(StrError(Box::from(err))))
    }
}

impl From<sqlx::error::Error> for UnknownError {
    fn from(err: sqlx::error::Error) -> Self {
        Self::new(err.into())
    }
}

impl Piece for UnknownError {
    fn render(self, res: &mut Response) {
        let status = StatusError::internal_server_error();
        res.render(Json(ErrorResponse::from_status_error(&status, ())));
        res.set_status_error(status);
    }
}

pub mod app {
    use derive_more::Display;
    use salvo::{prelude::StatusError, writer::Json, Piece};
    use serde::Serialize;

    use super::{
        http::ErrorResponse,
        persistence::{MutationError, PersistenceError},
        resource::{ConflictError, NotFoundError, ValidationError},
        security::{AuthenticationError, ForbiddenError},
    };

    #[derive(Debug, Display, Serialize)]
    pub enum ApplicationError<R> {
        Authentication(AuthenticationError),
        Forbidden(ForbiddenError),
        Validation(ValidationError<R>),
        Conflict(ConflictError),
        NotFound(NotFoundError),
        // Domain errors
        // Operation(OperationError) -> 422 Unprocessable Entity
        Persistence(PersistenceError),
    }

    impl<R: std::fmt::Debug> std::error::Error for ApplicationError<R> {}

    impl<R> From<AuthenticationError> for ApplicationError<R> {
        fn from(err: AuthenticationError) -> Self {
            Self::Authentication(err)
        }
    }

    impl<R> From<ForbiddenError> for ApplicationError<R> {
        fn from(err: ForbiddenError) -> Self {
            Self::Forbidden(err)
        }
    }

    impl<R> From<ValidationError<R>> for ApplicationError<R> {
        fn from(err: ValidationError<R>) -> Self {
            Self::Validation(err)
        }
    }

    impl<R> From<ConflictError> for ApplicationError<R> {
        fn from(err: ConflictError) -> Self {
            Self::Conflict(err)
        }
    }

    impl<R> From<NotFoundError> for ApplicationError<R> {
        fn from(err: NotFoundError) -> Self {
            Self::NotFound(err)
        }
    }

    impl<R> From<PersistenceError> for ApplicationError<R> {
        fn from(err: PersistenceError) -> Self {
            Self::Persistence(err)
        }
    }

    impl<R> From<MutationError> for ApplicationError<R> {
        fn from(err: MutationError) -> Self {
            match err {
                MutationError::Persistence(err) => err.into(),
                MutationError::Conflict(err) => err.into(),
            }
        }
    }

    impl<R: Serialize + Send> Piece for ApplicationError<R> {
        fn render(self, res: &mut salvo::Response) {
            let status = match &self {
                ApplicationError::Persistence(_) => StatusError::service_unavailable(),
                ApplicationError::Validation(_) => StatusError::bad_request(),
                ApplicationError::Authentication(_) => StatusError::unauthorized(),
                ApplicationError::Forbidden(_) => StatusError::forbidden(),
                ApplicationError::Conflict(_) => StatusError::conflict(),
                ApplicationError::NotFound(_) => StatusError::not_found(),
            };
            res.render(Json(ErrorResponse::from_status_error(&status, self)));
            res.set_status_error(status);
        }
    }
}

// TODO: remove, use std::io::Error instead
pub mod service {
    use derive_more::Display;

    use crate::error::UnknownError;

    // TODO: replace DispatchError by std::io::Error
    #[derive(Debug, Display)]
    pub enum DispatchError {
        #[display(fmt = "Dispatched operation timed out in {_0:?}")]
        Timeout(Option<std::time::Duration>),
        #[display(fmt = "Invalid input {_0:?}")]
        InvalidInput(Option<UnknownError>),
        #[display(fmt = "IO error dispatching {_0}")]
        IO(std::io::Error),
        #[display(fmt = "Unknown dispatch error {_0}")]
        Unknown(UnknownError),
    }

    impl std::error::Error for DispatchError {}
}

pub mod persistence {
    use std::{error, io};

    use derive_more::Display;
    use serde::Serialize;

    use super::{resource::ConflictError, service::DispatchError, UnknownError};

    pub type SqlState = String;

    #[derive(Debug, Display)]
    pub enum PersistenceError {
        #[display(fmt = "database persistence error: SQLSTATE {_0:?}")]
        Database(Option<SqlState>),
        #[display(fmt = "persistence layer connection error: {_0}")]
        Connection(DispatchError),
        #[display(fmt = "PersistenceError data not found")]
        NotFound,
        #[display(fmt = "PersistenceError decoding data")]
        DecodeData,
        #[display(fmt = "PersistenceError data migration")]
        DataMigration,
        #[display(fmt = "unknown persistence error: {_0}")]
        Unknown(UnknownError),
    }

    impl error::Error for PersistenceError {}

    impl Serialize for PersistenceError {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            serializer.serialize_none()
        }
    }

    type SqlxError = sqlx::error::Error;

    impl From<SqlxError> for PersistenceError {
        fn from(err: SqlxError) -> Self {
            tracing::error!(target = "database", cause = %err);
            match err {
                SqlxError::Configuration(_) => {
                    Self::Connection(DispatchError::IO(io::ErrorKind::InvalidInput.into()))
                }
                SqlxError::Database(db) => Self::Database(db.code().map(|code| code.into())),
                SqlxError::Io(io) => Self::Connection(DispatchError::IO(io)),
                SqlxError::Tls(_) => {
                    Self::Connection(DispatchError::IO(io::ErrorKind::ConnectionRefused.into()))
                }
                SqlxError::Protocol(msg) => Self::Connection(DispatchError::IO(io::Error::new(
                    io::ErrorKind::InvalidData,
                    msg,
                ))),
                SqlxError::RowNotFound => Self::NotFound,
                SqlxError::TypeNotFound { .. } => Self::DecodeData,
                SqlxError::ColumnIndexOutOfBounds { .. } => Self::DecodeData,
                SqlxError::ColumnNotFound(_) => Self::NotFound,
                SqlxError::ColumnDecode { .. } => Self::DecodeData,
                SqlxError::Decode(_) => Self::DecodeData,
                SqlxError::PoolTimedOut => Self::Connection(DispatchError::Timeout(None)),
                SqlxError::PoolClosed => {
                    Self::Connection(DispatchError::IO(io::ErrorKind::NotConnected.into()))
                }
                SqlxError::WorkerCrashed => {
                    panic!("PANIC! sqlx background worker error: {err}");
                }
                SqlxError::Migrate(_) => Self::DataMigration,
                _ => PersistenceError::Unknown(err.into()),
            }
        }
    }

    #[derive(Debug, Display)]
    pub enum MutationError {
        Persistence(PersistenceError),
        Conflict(ConflictError),
    }

    impl From<PersistenceError> for MutationError {
        fn from(err: PersistenceError) -> Self {
            Self::Persistence(err)
        }
    }

    impl From<ConflictError> for MutationError {
        fn from(err: ConflictError) -> Self {
            Self::Conflict(err)
        }
    }

    impl error::Error for MutationError {}
}

pub mod resource {
    use derive_more::{Display, Error};
    use serde::{Deserialize, Serialize};
    use uuid::Uuid;

    use crate::base::ResourceID;

    #[derive(Debug, Display, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub enum ValidationErrorKind {
        /// Unexpected properties.
        #[display(fmt = "Validation error kind: additional_properties {_0:?}")]
        AdditionalProperties(Vec<String>),
        /// Not enough properties in an object.
        MinProperties(u64),
        /// Too many properties in an object.
        MaxProperties(u64),
        /// Object property names are invalid.
        #[display(fmt = "Validation error kind: property_name {_0}")]
        PropertyName(String),
        /// When a required property is missing.
        Required,

        /// Maximum inclusive string length.
        MaxLength(u64),
        /// Minimum inclusive string length.
        MinLength(u64),
        /// When the input doesn't match to a pattern.
        Pattern(String),
        /// When the input match to a pattern.
        NegativePattern(String),

        /// Minimum inclusive number of items in an array exceeded.
        MinItems(u64),
        /// Maximum inclusive number of items in an array exceeded.
        MaxItems(u64),

        /// Inclusive lower bound exceeded.
        Minimum(u64),
        /// Inclusive higher bound exceeded.
        Maximum(u64),
        /// When some number is not a multiple of another number.
        MultipleOf(i64),
        /// When some number is not positive.
        Positive,
        /// When some number is not negative.
        Negative,

        /// The input value doesn't match any of specified options.
        UnknownVariant,
        /// The input value doesn't match one or multiple required types.
        InvalidType,
        /// When the value requires some aditional verification.
        Unverified,
        /// Duplicated input value.
        Duplicated,
        /// Input value already exists.
        AlreadyExists,
        /// Input value was not found.
        NotFound,
        /// Generic kind.
        Invalid,
    }

    impl std::error::Error for ValidationErrorKind {}

    #[derive(Debug, Error, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct ValidationError<R> {
        /// Resource value
        pub resource: R,
        /// Name of the resource
        pub resource_type: &'static str,
        /// Invalid resource fields
        pub fields: Vec<ValidationFieldError>,
    }

    impl<R> ValidationError<R> {
        pub fn from_resource(resource: R, fields: Vec<ValidationFieldError>) -> Self
        where
            R: ResourceID,
        {
            Self {
                resource,
                resource_type: R::resource_id(),
                fields,
            }
        }
    }

    impl<R> std::fmt::Display for ValidationError<R> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_fmt(format_args!(
                "Invalid resource {}, fields {:?}",
                self.resource_type, self.fields
            ))
        }
    }

    #[derive(Debug, Display, Error, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[display(fmt = "{path}: {value:?}, {kinds:?}")]
    pub struct ValidationFieldError {
        /// Resource field path with invalid value
        pub path: String,
        /// Displayed invalid value
        pub value: String,
        /// Value type id
        pub type_id: &'static str,
        /// Kinds of validation errors
        pub kinds: Vec<ValidationErrorKind>,
    }

    impl ValidationFieldError {
        pub fn from_resource<T>(
            value: String,
            path: String,
            kinds: Vec<ValidationErrorKind>,
        ) -> Self
        where
            T: ResourceID,
        {
            Self {
                path,
                type_id: T::resource_id(),
                value,
                kinds,
            }
        }

        pub fn new(
            type_id: &'static str,
            value: String,
            path: String,
            kinds: Vec<ValidationErrorKind>,
        ) -> Self {
            Self {
                path,
                type_id,
                value,
                kinds,
            }
        }
    }

    #[derive(Debug, Display, Clone, Error, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[display(fmt = "Conflicting resource {resource_type} of id {resource_id:?}")]
    pub struct ConflictError {
        /// Resource id
        pub resource_id: Option<Uuid>,
        /// Name of the resource
        pub resource_type: &'static str,
    }

    impl ConflictError {
        pub fn from_resource<T: ResourceID>(id: Option<Uuid>) -> Self {
            Self {
                resource_id: id,
                resource_type: T::resource_id(),
            }
        }
    }

    #[derive(Debug, Display, Clone, Error, PartialEq, Eq, Serialize, Deserialize)]
    #[display(fmt = "Resource {resource_type} not found")]
    pub struct NotFoundError {
        /// Resource id
        pub resource_id: Uuid,
        /// Name of the resource
        pub resource_type: &'static str,
    }

    impl NotFoundError {
        pub fn from_resource<T: ResourceID>(id: Uuid) -> Self {
            Self {
                resource_id: id,
                resource_type: T::resource_id(),
            }
        }
    }
}

pub mod security {
    use derive_more::Display;
    use serde::Serialize;

    use crate::domain::datatype::security::{PasswordHashError, TokenEncryptionError};

    /// Authentication error.
    ///
    /// The user is not authenticated to access the resource.
    #[derive(Debug, Display, Serialize)]
    pub enum AuthenticationError {
        /// Attempt to authenticate with invalid credentials.
        #[display(fmt = "invalid_credential")]
        InvalidCredential,

        /// Authentication token is not present.
        #[display(fmt = "token_not_present")]
        TokenNotPresent,

        /// Authentication token is malformatted.
        ///
        /// The token is no formated as the required authentication scheme
        #[display(fmt = "malformatted_token")]
        MalformattedToken,

        /// Authentication token is invalid.
        #[display(fmt = "invalid_token")]
        InvalidToken,
    }

    #[derive(Debug, Display, Serialize)]
    pub enum ForbiddenError {
        /// Access denied.
        ///
        /// The user is authenticated, however does not have access to the requested resource.
        #[display(fmt = "access_denied")]
        AccessDenied,

        /// Forbidden access due invalid credential.
        ///
        /// Authentication credentials is required to grant access, but invalid credentials was send.
        #[display(fmt = "invalid_credential")]
        InvalidCredential,
    }

    impl From<TokenEncryptionError> for AuthenticationError {
        fn from(_: TokenEncryptionError) -> Self {
            Self::InvalidToken
        }
    }

    impl From<PasswordHashError> for AuthenticationError {
        fn from(_: PasswordHashError) -> Self {
            Self::InvalidCredential
        }
    }

    impl From<PasswordHashError> for ForbiddenError {
        fn from(_: PasswordHashError) -> Self {
            Self::InvalidCredential
        }
    }
}

pub mod http {
    use derive_more::{Display, Error};
    use salvo::{http::ParseError, prelude::StatusError, writer::Json, Piece, Response};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Display, Clone, Error, Serialize, Deserialize)]
    pub enum BadRequest {
        InvalidContent,
    }

    #[derive(Debug, Display, Clone, Error, Serialize, Deserialize)]
    #[display(fmt = "Response error: {title}, {message}")]
    pub struct ErrorResponse<T> {
        pub title: String,
        pub message: String,
        pub error: T,
    }

    impl<T> ErrorResponse<T> {
        pub fn from_status_error(status: &StatusError, err: T) -> Self {
            Self {
                title: status.name.clone(),
                message: status
                    .summary
                    .clone()
                    .unwrap_or_else(|| status.name.clone()),
                error: err,
            }
        }
    }

    impl From<ParseError> for BadRequest {
        fn from(_: ParseError) -> Self {
            BadRequest::InvalidContent
        }
    }

    impl Piece for BadRequest {
        fn render(self, res: &mut Response) {
            let status = StatusError::bad_request();
            res.render(Json(ErrorResponse::from_status_error(&status, self)));
            res.set_status_error(status);
        }
    }
}
