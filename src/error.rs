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

impl From<pg_tokio::Error> for UnknownError {
    fn from(err: pg_tokio::Error) -> Self {
        UnknownError(err.into())
    }
}

impl From<pg_pool::PoolError> for UnknownError {
    fn from(err: pg_pool::PoolError) -> Self {
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

pub mod app {
    use derive_more::Display;

    #[derive(Debug, Display)]
    pub enum OperationLayer {
        Domain,
        App,
        Infra,
    }

    #[derive(Debug, Display)]
    #[display(fmt = "OperationError in context {context}. {details}")]
    pub struct OperationError<T: std::error::Error> {
        pub(super) context: &'static str,
        pub(super) details: T,
        pub(super) layer: OperationLayer,
    }

    impl<T: std::error::Error> OperationError<T> {
        pub fn domain(context: &'static str, err: T) -> Self {
            Self {
                context,
                layer: OperationLayer::Domain,
                details: err,
            }
        }

        pub fn app(context: &'static str, err: T) -> Self {
            Self {
                context,
                layer: OperationLayer::App,
                details: err,
            }
        }

        pub fn infra(context: &'static str, err: T) -> Self {
            Self {
                context,
                layer: OperationLayer::Infra,
                details: err,
            }
        }
    }

    impl<T: std::error::Error> std::error::Error for OperationError<T> {}
}

pub mod service {
    use derive_more::Display;

    use crate::error::UnknownError;

    #[derive(Debug, Display)]
    pub enum DispatchError {
        #[display(fmt = "Dispatched operation timed out {_0}")]
        Timeout(UnknownError),
        #[display(fmt = "Invalid request {_0}")]
        InvalidRequest(UnknownError),
        #[display(fmt = "IO error dispatching {_0}")]
        IO(UnknownError),
        #[display(fmt = "Unknown dispatch error {_0}")]
        Unknown(UnknownError),
    }

    impl std::error::Error for DispatchError {}

    #[derive(Debug, Display)]
    pub enum ResponseError {
        #[display(fmt = "Unknown response error {_0}")]
        Unknown(UnknownError),
    }
}

pub mod storage {
    use derive_more::{Display, Error};
    use salvo::{prelude::StatusError, writer::Json, Piece, Response};

    use super::{
        http::ErrorResponse,
        service::{DispatchError, ResponseError},
        UnknownError,
    };

    #[derive(Debug, Display, Error)]
    pub enum DatabaseError {
        #[display(fmt = "database error: {_0}")]
        Db(pg_tokio::error::DbError),
        #[display(fmt = "database connection error: {_0}")]
        Connection(DispatchError),
        #[display(fmt = "unknown database error: {_0}")]
        Unknown(UnknownError),
    }

    impl From<pg_tokio::Error> for DatabaseError {
        fn from(err: pg_tokio::Error) -> Self {
            if let Some(db_err) = err.as_db_error() {
                tracing::error!("DatabaseError {db_err:?}");
                return DatabaseError::Db(db_err.clone());
            }

            tracing::error!("UnknownDatabaseError {err:?}");
            DatabaseError::Unknown(err.into())
        }
    }

    impl From<pg_pool::PoolError> for DatabaseError {
        fn from(err: pg_pool::PoolError) -> Self {
            match err {
                pg_pool::PoolError::Backend(back) => {
                    if let Some(db) = back.as_db_error() {
                        tracing::error!("Error retrieving database connection from pool {db:?}");
                        return Self::Db(db.clone());
                    }

                    tracing::error!("Error retrieving database connection from pool {back:?}");
                    Self::Connection(DispatchError::Unknown(back.into()))
                }
                pg_pool::PoolError::Timeout(_) => {
                    tracing::error!("TimeoutError retrieving database connection from pool");
                    Self::Connection(DispatchError::Timeout(err.into()))
                }
                pg_pool::PoolError::NoRuntimeSpecified => {
                    panic!("Error retrieving database connection from pool: No runtime specified");
                }
                err => {
                    tracing::error!("Error retrieving database connection from pool {err:?}");
                    Self::Connection(DispatchError::Unknown(err.into()))
                }
            }
        }
    }

    impl Piece for DatabaseError {
        fn render(self, res: &mut Response) {
            let status = StatusError::service_unavailable();
            res.render(Json(ErrorResponse::from_status_error(&status, ())));
            res.set_status_error(status);
        }
    }

    #[derive(Debug, Display)]
    pub enum StorageError {
        DispatchFailure(DispatchError),
        InvalidResponse(ResponseError),
        ObjectNotFound,
        ObjectArchived,
        Unknown(UnknownError),
    }

    impl std::error::Error for StorageError {}

    impl From<UnknownError> for StorageError {
        fn from(err: UnknownError) -> Self {
            Self::Unknown(err)
        }
    }
}

pub mod resource {
    use derive_more::{Display, Error};
    use uuid::Uuid;

    use crate::base::ResourceID;

    #[derive(Debug, Display, Clone, PartialEq, Eq, Hash)]
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

    // impl From<email_address::Error> for ValidationErrorKind {
    //     fn from(_: email_address::Error) -> Self {
    //         Self::Pattern("email".into())
    //     }
    // }

    #[derive(Debug, Display, Error, Clone, PartialEq, Eq, Hash)]
    #[display(fmt = "Invalid resource {resource_type}, fields {fields:?}")]
    pub struct ValidationError<R> {
        /// Resource value
        pub resource: R,
        /// Name of the resource
        pub resource_type: String,
        /// Invalid resource fields
        pub fields: Vec<ValidationFieldError>,
    }

    #[derive(Debug, Display, Error, Clone, PartialEq, Eq, Hash)]
    #[display(fmt = "{path}: {value:?}, {kinds:?}")]
    pub struct ValidationFieldError {
        /// Resource field path with invalid value
        pub path: String,
        /// Displayed invalid value
        pub value: String,
        /// Value type id
        pub type_id: String,
        /// Kinds of validation errors
        pub kinds: Vec<ValidationErrorKind>,
    }

    impl ValidationFieldError {
        pub fn from_field<T>(value: String, kinds: Vec<ValidationErrorKind>, path: String) -> Self
        where
            T: ResourceID,
        {
            Self {
                kinds,
                path,
                type_id: T::resource_id().into(),
                value,
            }
        }

        pub fn from_required_field<T>(value: String, path: String) -> Self
        where
            T: ResourceID,
        {
            Self {
                kinds: vec![ValidationErrorKind::Required],
                path,
                type_id: T::resource_id().into(),
                value,
            }
        }

        pub fn from_unknown_variant_field<T>(value: String, path: String) -> Self
        where
            T: ResourceID,
        {
            Self {
                kinds: vec![ValidationErrorKind::UnknownVariant],
                path,
                type_id: T::resource_id().into(),
                value,
            }
        }
    }

    #[derive(Debug, Display, Clone, Error)]
    #[display(fmt = "Conflicting resource {resource_type} of id {resource_id}")]
    pub struct ConflictError<R> {
        /// Resource id
        pub resource_id: Uuid,
        /// Name of the resource
        pub resource_type: String,
        /// Stabe resource already found
        pub stable: R,
        /// New conflicting resource
        pub conflict: Option<R>,
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
