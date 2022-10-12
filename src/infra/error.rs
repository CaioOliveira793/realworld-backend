use derive_more::Display;
use salvo::{prelude::StatusError, writer::Json, Piece, Response};

use self::http::ErrorResponse;

#[derive(Debug, Display)]
pub struct UnknownError(Box<dyn std::error::Error + Send + Sync + 'static>);

impl std::error::Error for UnknownError {}

impl From<tokio_postgres::Error> for UnknownError {
    fn from(err: tokio_postgres::Error) -> Self {
        UnknownError(err.into())
    }
}

impl Piece for UnknownError {
    fn render(self, res: &mut Response) {
        let status = StatusError::internal_server_error();
        res.render(Json(ErrorResponse::from_status_error(&status, ())));
        res.set_status_error(status);
    }
}

pub mod storage {
    use derive_more::{Display, Error};
    use salvo::{prelude::StatusError, writer::Json, Piece, Response};

    use super::{http::ErrorResponse, UnknownError};

    pub type DbError = tokio_postgres::error::DbError;

    #[derive(Debug, Display, Error)]
    pub enum RepositoryError {
        #[display(fmt = "database error: {_0}")]
        Db(DbError),
        #[display(fmt = "unknown database error: {_0}")]
        Unknown(UnknownError),
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
            let status = StatusError::service_unavailable();
            res.render(Json(ErrorResponse::from_status_error(&status, ())));
            res.set_status_error(status);
        }
    }
}

pub mod resource {
    use std::any::Any;

    use derive_more::{Display, Error};

    #[derive(Debug, Display, Clone, PartialEq)]
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
        Required(String),

        /// String is too long.
        MaxLength(u64),
        /// String is too short.
        MinLength(u64),
        /// When the input doesn't match to a pattern.
        Pattern(String),

        /// Too many items in an array.
        MaxItems(u64),
        /// Too few items in an array.
        MinItems(u64),

        /// Value is too small.
        Minimum(u64),
        /// Value is too large.
        Maximum(u64),
        /// When some number is not a multiple of another number.
        MultipleOf(f64),

        /// The input value doesn't match any of specified options.
        UnknownVariant,
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

    #[derive(Debug, Display, Clone, Error)]
    #[display(fmt = "Invalid resource {resource_type}, fields {fields:?}")]
    pub struct ValidationError<'f, R> {
        /// Resource value
        pub resource_value: R,
        /// Name of the resource
        pub resource_type: String,
        /// Invalid resource fields
        pub fields: Vec<ValidationFieldError<'f>>,
    }

    #[derive(Debug, Display, Clone, Error)]
    #[display(fmt = "{path}: {value:?}, {kinds:?}")]
    pub struct ValidationFieldError<'f> {
        /// Resource field path with invalid value
        pub path: String,
        /// Invalid value
        pub value: Box<&'f dyn Any>,
        /// Value type id
        pub type_id: String,
        /// Kinds of validation errors
        pub kinds: Vec<ValidationErrorKind>,
    }

    #[derive(Debug, Display, Clone, Error)]
    #[display(fmt = "Conflicting resource {resource_type} of id {resource_id}")]
    pub struct ConflictError<R> {
        /// Resource id
        pub resource_id: String,
        /// Name of the resource
        pub resource_type: String,
        /// Stabe resource already found
        pub stable: R,
        /// New conflicting resource
        pub conflict: Option<R>,
    }

    impl std::error::Error for ValidationErrorKind {}
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
                message: status.summary.clone().unwrap_or(status.name.clone()),
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
