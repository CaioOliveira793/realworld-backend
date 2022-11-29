use std::str::FromStr;

use derive_more::Display;

use crate::{
    base::ResourceID,
    error::resource::{ValidationErrorKind, ValidationFieldError},
};

#[derive(Debug, Display, Clone, PartialEq, Eq)]
pub enum PasswordHashAlgorithm {
    #[display(fmt = "argon2d")]
    Argon2d,
    #[display(fmt = "argon2i")]
    Argon2i,
    #[display(fmt = "argon2id")]
    Argon2id,
    #[display(fmt = "2b")]
    Bcrypt,
}

impl PasswordHashAlgorithm {
    pub fn as_str(&self) -> &'static str {
        match self {
            PasswordHashAlgorithm::Argon2d => "argon2d",
            PasswordHashAlgorithm::Argon2i => "argon2i",
            PasswordHashAlgorithm::Argon2id => "argon2id",
            PasswordHashAlgorithm::Bcrypt => "2b",
        }
    }
}

impl ResourceID for PasswordHashAlgorithm {
    fn resource_id() -> &'static str {
        "base::password_hash_algorithm"
    }
}

impl FromStr for PasswordHashAlgorithm {
    type Err = ValidationFieldError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "2b" | "2a" => Ok(Self::Bcrypt),
            "argon2d" => Ok(Self::Argon2d),
            "argon2i" => Ok(Self::Argon2i),
            "argon2id" => Ok(Self::Argon2id),
            _ => Err(ValidationFieldError::from_resource::<Self>(
                s.into(),
                String::new(),
                vec![ValidationErrorKind::UnknownVariant],
            )),
        }
    }
}

impl TryFrom<password_hash::Ident<'_>> for PasswordHashAlgorithm {
    type Error = ValidationFieldError;

    fn try_from(value: password_hash::Ident) -> Result<Self, Self::Error> {
        Self::from_str(value.as_str())
    }
}

impl From<argon2::Algorithm> for PasswordHashAlgorithm {
    fn from(algo: argon2::Algorithm) -> Self {
        match algo {
            argon2::Algorithm::Argon2d => Self::Argon2d,
            argon2::Algorithm::Argon2i => Self::Argon2i,
            argon2::Algorithm::Argon2id => Self::Argon2id,
        }
    }
}

impl<'a> From<&'a PasswordHashAlgorithm> for password_hash::Ident<'a> {
    fn from(algo: &'a PasswordHashAlgorithm) -> Self {
        Self::new(algo.as_str())
            .expect("Expect PasswordHashAlgorithm to have a valid symbolic name")
    }
}

/// Password hash.
///
/// A parsed representation of a PHC string as described in the [PHC string format specification][1].
///
/// PHC strings have the following format:
///
/// ```text
/// $<id>[$v=<version>][$<param>=<value>(,<param>=<value>)*][$<salt>[$<hash>]]
/// ```
///
/// where:
///
/// - `<id>` is the symbolic name for the function
/// - `<version>` is the algorithm version
/// - `<param>` is a parameter name
/// - `<value>` is a parameter value
/// - `<salt>` is an encoding of the salt
/// - `<hash>` is an encoding of the hash output
///
/// The string is then the concatenation, in that order, of:
///
/// - a `$` sign;
/// - the function symbolic name;
/// - optionally, a `$` sign followed by the algorithm version with a `v=version` format;
/// - optionally, a `$` sign followed by one or several parameters, each with a `name=value` format;
///   the parameters are separated by commas;
/// - optionally, a `$` sign followed by the (encoded) salt value;
/// - optionally, a `$` sign followed by the (encoded) hash output (the hash output may be present
///   only if the salt is present).
///
/// [1]: https://github.com/P-H-C/phc-string-format/blob/master/phc-sf-spec.md#specification
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PasswordHash {
    /// Password hashing algorithm identifier.
    ///
    /// This corresponds to the `<id>` field in a PHC string, a.k.a. the
    /// symbolic name for the function.
    pub algorithm: PasswordHashAlgorithm,

    /// Optional version field.
    ///
    /// This corresponds to the `<version>` field in a PHC string.
    pub version: Option<u32>,

    /// Algorithm-specific parameters.
    ///
    /// This corresponds to the set of `$<param>=<value>(,<param>=<value>)*`
    /// name/value pairs in a PHC string.
    pub params: password_hash::ParamsString,

    /// Salt string for personalizing a password hash output.
    ///
    /// This corresponds to the `<salt>` value in a PHC string.
    pub salt: Option<password_hash::SaltString>,

    /// Password hashing function Output, a.k.a. hash/digest.
    ///
    /// This corresponds to the `<hash>` output in a PHC string.
    pub hash: Option<password_hash::Output>,
}

impl PasswordHash {
    pub const SEPARATOR: char = '$';
}

impl ResourceID for PasswordHash {
    fn resource_id() -> &'static str {
        "base::password_hash"
    }
}

impl FromStr for PasswordHash {
    type Err = ValidationFieldError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let hash =
            password_hash::PasswordHash::parse(s, password_hash::Encoding::B64).map_err(|_| {
                Self::Err::from_resource::<Self>(
                    s.into(),
                    String::new(),
                    vec![ValidationErrorKind::Invalid],
                )
            })?;

        Ok(Self {
            algorithm: hash.algorithm.try_into()?,
            version: hash.version,
            params: hash.params,
            salt: hash.salt.map(|salt| {
                password_hash::SaltString::new(salt.as_str()).expect("Expected a valid Salt")
            }),
            hash: hash.hash,
        })
    }
}

impl std::fmt::Display for PasswordHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "${}", self.algorithm)?;

        if let Some(version) = self.version {
            write!(f, "$v={}", version)?
        }

        if !self.params.is_empty() {
            write!(f, "${}", self.params)?;
        }

        if let Some(ref salt) = self.salt {
            write!(f, "${}", salt)?;
        }

        if let Some(hash) = self.hash {
            write!(f, "${}", hash)?;
        }

        Ok(())
    }
}

impl TryFrom<&PasswordHash> for argon2::Params {
    type Error = PasswordHashError;

    fn try_from(hash: &PasswordHash) -> Result<Self, Self::Error> {
        let mut builder = argon2::ParamsBuilder::new();

        for (ident, value) in hash.params.iter() {
            match ident.as_str() {
                "m" => {
                    builder.m_cost(value.decimal()?)?;
                }
                "t" => {
                    builder.t_cost(value.decimal()?)?;
                }
                "p" => {
                    builder.p_cost(value.decimal()?)?;
                }
                "keyid" => {
                    builder.keyid(value.as_bytes())?;
                }
                "data" => {
                    builder.data(value.as_bytes())?;
                }
                _ => (),
            }
        }

        if let Some(output) = &hash.hash {
            builder.output_len(output.len())?;
        }

        Ok(builder.try_into()?)
    }
}

impl<DB: sqlx::Database> sqlx::Type<DB> for PasswordHash
where
    str: sqlx::Type<DB>,
{
    fn compatible(ty: &DB::TypeInfo) -> bool {
        <&str as sqlx::Type<DB>>::compatible(ty)
    }

    fn type_info() -> <DB as sqlx::Database>::TypeInfo {
        <&str as sqlx::Type<DB>>::type_info()
    }
}

pub enum PasswordHashError {
    /// Unsupported Algorithm.
    UnsupportedAlgorithm,

    /// Invalid password.
    InvalidPassword,

    /// Invalid password hash.
    InvalidPasswordHash,

    /// Cryptographic error.
    Cryptographic,

    /// Error in the hasher configuration.
    Config,

    /// Error in the hasher configuration.
    Unknown,
}

impl From<password_hash::Error> for PasswordHashError {
    fn from(err: password_hash::Error) -> Self {
        match err {
            password_hash::Error::Algorithm => Self::UnsupportedAlgorithm,
            password_hash::Error::B64Encoding(_) => Self::InvalidPasswordHash,
            password_hash::Error::Crypto => Self::Cryptographic,
            password_hash::Error::OutputTooShort => Self::Cryptographic,
            password_hash::Error::OutputTooLong => Self::Cryptographic,
            password_hash::Error::ParamNameDuplicated => Self::Config,
            password_hash::Error::ParamNameInvalid => Self::Config,
            password_hash::Error::ParamValueInvalid(_) => Self::Config,
            password_hash::Error::ParamsMaxExceeded => Self::Config,
            password_hash::Error::Password => Self::InvalidPassword,
            password_hash::Error::PhcStringInvalid => Self::InvalidPasswordHash,
            password_hash::Error::PhcStringTooShort => Self::InvalidPasswordHash,
            password_hash::Error::PhcStringTooLong => Self::InvalidPasswordHash,
            password_hash::Error::SaltInvalid(_) => Self::Config,
            password_hash::Error::Version => Self::UnsupportedAlgorithm,
            _ => Self::Unknown,
        }
    }
}

impl From<argon2::Error> for PasswordHashError {
    fn from(err: argon2::Error) -> Self {
        match err {
            argon2::Error::AdTooLong => Self::Config,
            argon2::Error::AlgorithmInvalid => Self::UnsupportedAlgorithm,
            argon2::Error::B64Encoding(_) => Self::Config,
            argon2::Error::KeyIdTooLong => Self::InvalidPasswordHash,
            argon2::Error::MemoryTooLittle => Self::Config,
            argon2::Error::MemoryTooMuch => Self::Config,
            argon2::Error::OutputTooShort => Self::Config,
            argon2::Error::OutputTooLong => Self::Config,
            argon2::Error::PwdTooLong => Self::InvalidPassword,
            argon2::Error::SaltTooShort => Self::Config,
            argon2::Error::SaltTooLong => Self::Config,
            argon2::Error::SecretTooLong => Self::Config,
            argon2::Error::ThreadsTooFew => Self::Config,
            argon2::Error::ThreadsTooMany => Self::Config,
            argon2::Error::TimeTooSmall => Self::Config,
            argon2::Error::VersionInvalid => Self::UnsupportedAlgorithm,
        }
    }
}
