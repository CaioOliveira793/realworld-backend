use std::fmt;
use std::str::FromStr;
use std::time::Duration;

use derive_more::Display;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::base::ResourceID;
use crate::error::resource::{ValidationErrorKind, ValidationFieldError};

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

impl TryFrom<PasswordHashAlgorithm> for argon2::Algorithm {
    type Error = PasswordHashError;

    fn try_from(value: PasswordHashAlgorithm) -> Result<Self, Self::Error> {
        match value {
            PasswordHashAlgorithm::Argon2d => Ok(Self::Argon2d),
            PasswordHashAlgorithm::Argon2i => Ok(Self::Argon2i),
            PasswordHashAlgorithm::Argon2id => Ok(Self::Argon2id),
            PasswordHashAlgorithm::Bcrypt => Err(Self::Error::UnsupportedAlgorithm),
        }
    }
}

impl<'a> From<&'a PasswordHashAlgorithm> for password_hash::Ident<'a> {
    fn from(algo: &'a PasswordHashAlgorithm) -> Self {
        Self::new(algo.as_str())
            .expect("Expect `PasswordHashAlgorithm` to have a valid symbolic name")
    }
}

/// Argon2 algorithm version
#[derive(Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord)]
pub enum Argon2Version {
    V16 = 16,
    V19 = 19,
}

impl ResourceID for Argon2Version {
    fn resource_id() -> &'static str {
        "base::argon2_version"
    }
}

impl From<Argon2Version> for u32 {
    fn from(ver: Argon2Version) -> Self {
        ver as u32
    }
}

impl TryFrom<u32> for Argon2Version {
    type Error = ValidationFieldError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            16 => Ok(Self::V16),
            19 => Ok(Self::V19),
            _ => Err(Self::Error::from_resource::<Self>(
                value.to_string(),
                String::new(),
                vec![ValidationErrorKind::UnknownVariant],
            )),
        }
    }
}

/// Argon2 password hash parameters.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Argon2Params {
    /// Memory size, expressed in kilobytes, between 1 and (2^32)-1.
    ///
    /// Value is an integer in decimal (1 to 10 digits).
    memory_cost: u32,

    /// Number of iterations, between 1 and (2^32)-1.
    ///
    /// Value is an integer in decimal (1 to 10 digits).
    iteration_cost: u32,

    /// Degree of parallelism, between 1 and 255.
    ///
    /// Value is an integer in decimal (1 to 3 digits).
    parallelism: u32,
}

impl ResourceID for Argon2Params {
    fn resource_id() -> &'static str {
        "base::argon2_parameter"
    }
}

impl From<Argon2Params> for password_hash::ParamsString {
    fn from(params: Argon2Params) -> Self {
        let mut output = password_hash::ParamsString::new();
        output
            .add_decimal("m", params.memory_cost)
            .expect("Expected to add memory (m) parameter to the argon2 ParamString");
        output
            .add_decimal("t", params.iteration_cost)
            .expect("Expected to add iteration cost (t) parameter to the argon2 ParamString");
        output
            .add_decimal("p", params.parallelism)
            .expect("Expected to add parallelism (p) parameter to the argon2 ParamString");
        output
    }
}

pub type PasswordParams = password_hash::ParamsString;
pub type SaltString = password_hash::SaltString;
pub type OutputHash = password_hash::Output;

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
    algorithm: PasswordHashAlgorithm,

    /// Optional version field.
    ///
    /// This corresponds to the `<version>` field in a PHC string.
    version: Option<u32>,

    /// Algorithm-specific parameters.
    ///
    /// This corresponds to the set of `$<param>=<value>(,<param>=<value>)*`
    /// name/value pairs in a PHC string.
    params: PasswordParams,

    /// Salt string for personalizing a password hash output.
    ///
    /// This corresponds to the `<salt>` value in a PHC string.
    salt: Option<SaltString>,

    /// Password hashing function Output, a.k.a. hash/digest.
    ///
    /// This corresponds to the `<hash>` output in a PHC string.
    hash: Option<OutputHash>,
}

impl PasswordHash {
    pub const SEPARATOR: char = '$';

    pub fn new(
        algorithm: PasswordHashAlgorithm,
        version: Option<u32>,
        params: PasswordParams,
        salt: Option<SaltString>,
        hash: Option<OutputHash>,
    ) -> Self {
        Self {
            algorithm,
            version,
            params,
            salt,
            hash,
        }
    }

    pub fn new_argon2(
        algorithm: argon2::Algorithm,
        version: Argon2Version,
        params: Argon2Params,
        salt: Option<SaltString>,
        hash: Option<OutputHash>,
    ) -> Self {
        Self {
            algorithm: algorithm.into(),
            version: Some(version.into()),
            params: params.into(),
            salt,
            hash,
        }
    }

    pub fn new_bcrypt(cost: u32, salt: Option<SaltString>, hash: Option<OutputHash>) -> Self {
        let mut params = password_hash::ParamsString::new();
        params
            .add_decimal("cost", cost)
            .expect("Expected to add cost (c) parameter to the bcrypt ParamString");
        Self {
            algorithm: PasswordHashAlgorithm::Bcrypt,
            version: None,
            params,
            salt,
            hash,
        }
    }

    pub fn algorithm(&self) -> &PasswordHashAlgorithm {
        &self.algorithm
    }

    pub fn version(&self) -> Option<u32> {
        self.version
    }

    pub fn params(&self) -> &PasswordParams {
        &self.params
    }

    pub fn salt(&self) -> &Option<SaltString> {
        &self.salt
    }

    pub fn hash(&self) -> &Option<OutputHash> {
        &self.hash
    }
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
                    String::new(),
                    String::new(),
                    vec![ValidationErrorKind::Invalid],
                )
            })?;

        Ok(Self {
            algorithm: hash.algorithm.try_into()?,
            version: hash.version,
            params: hash.params,
            salt: hash
                .salt
                .map(|salt| SaltString::new(salt.as_str()).expect("Expected a valid Salt")),
            hash: hash.hash,
        })
    }
}

impl fmt::Display for PasswordHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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

#[cfg(test)]
mod password_hash_test {
    use std::str::FromStr;

    use pretty_assertions::assert_eq;

    use super::PasswordHash;

    #[test]
    fn parse_and_serialize() {
        let pwds = [
            "$2b$c=10$b0tmWkRkdUNuN1ZsbVVSSw$JKBjx7b7p7pb0SGk0bKwAg",
            "$2b$c=11$UnhIaHlwdDRkQm1QN3dFRA$IhqNdiDUZbYQpWFJHTPYbw",
            "$argon2i$v=19$m=16,t=3,p=1$cG5nRUQ1VDgxT1FUa296bA$Ju09TJ75fE0J6rSZEEwOGg",
            "$argon2d$v=19$m=16,t=3,p=1$dXVwdmdFZm1xOU44YWdFZQ$2BRumjvZnUsQZHXPlqqcPA",
            "$argon2id$v=19$m=16,t=3,p=1$TE1LcnNPbTVEcnNQYTBPUA$2JYnsTwG5Zu17cIWiaAxnA",
        ];

        for pwd in pwds {
            let pass_hash =
                PasswordHash::from_str(pwd).expect("Expect to parse a valid encoded password");

            assert_eq!(
                pwd,
                pass_hash.to_string().as_str(),
                "Expect to display the same as encoded"
            );
        }
    }
}

#[derive(Debug, Display, PartialEq, Eq)]
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

/// Token issuer
#[derive(Debug, PartialEq, Eq)]
pub struct TokenIssuer;

impl TokenIssuer {
    pub fn as_str() -> &'static str {
        "conduit.blog.app"
    }
}

impl ResourceID for TokenIssuer {
    fn resource_id() -> &'static str {
        "base::token_issuer"
    }
}

impl fmt::Display for TokenIssuer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(Self::as_str())
    }
}

impl FromStr for TokenIssuer {
    type Err = ValidationFieldError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == Self::as_str() {
            return Ok(Self);
        }

        Err(Self::Err::from_resource::<Self>(
            s.into(),
            String::new(),
            vec![ValidationErrorKind::Invalid],
        ))
    }
}

impl Serialize for TokenIssuer {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(Self::as_str())
    }
}

impl<'de> Deserialize<'de> for TokenIssuer {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{Error, Unexpected};

        let s: &str = Deserialize::deserialize(deserializer)?;
        TokenIssuer::from_str(s)
            .map_err(|_| Error::invalid_value(Unexpected::Str(s), &Self::resource_id()))
    }
}

/// Token subject (sub)
///
/// Whom token refers to.
#[derive(Debug, PartialEq, Eq)]
pub enum TokenSubject {
    User(Uuid),
    Public,
}

impl ResourceID for TokenSubject {
    fn resource_id() -> &'static str {
        "base::token_subject"
    }
}

impl FromStr for TokenSubject {
    type Err = ValidationFieldError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "public" {
            return Ok(Self::Public);
        }

        if let Some(id_str) = s.strip_prefix("user:") {
            let id = Uuid::from_str(id_str).map_err(|_| {
                Self::Err::from_resource::<Self>(
                    s.into(),
                    String::new(),
                    vec![ValidationErrorKind::Pattern("^user:<uuid>$".into())],
                )
            })?;
            return Ok(Self::User(id));
        }

        Err(Self::Err::from_resource::<Self>(
            s.into(),
            String::new(),
            vec![ValidationErrorKind::UnknownVariant],
        ))
    }
}

impl fmt::Display for TokenSubject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenSubject::User(id) => write!(f, "user:{id}"),
            TokenSubject::Public => f.write_str("public"),
        }
    }
}

impl Serialize for TokenSubject {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.to_string().as_str())
    }
}

impl<'de> Deserialize<'de> for TokenSubject {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{Error, Unexpected};

        let s: &str = Deserialize::deserialize(deserializer)?;
        Self::from_str(s)
            .map_err(|_| Error::invalid_value(Unexpected::Str(s), &Self::resource_id()))
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenPayload<T> {
    /// Expiration time (as UTC timestamp in seconds)
    exp: u64,
    /// Issued at (as UTC timestamp in seconds)
    iat: u64,
    /// Issuer
    iss: TokenIssuer,
    /// Subject (whom token refers to)
    sub: TokenSubject,
    /// Associated data
    pub data: T,
}

impl<T> TokenPayload<T> {
    pub fn new(expiration: Duration, subject: TokenSubject, data: T) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();

        Self {
            exp: now + expiration.as_secs(),
            iat: now,
            iss: TokenIssuer,
            sub: subject,
            data,
        }
    }

    pub fn subject(&self) -> &TokenSubject {
        &self.sub
    }

    pub fn issuer(&self) -> &TokenIssuer {
        &self.iss
    }

    /// Time when the token was issued
    ///
    /// UTC timestamp in seconds
    pub fn issued_at(&self) -> u64 {
        self.iat
    }

    /// Time when the token will be expired
    ///
    /// UTC timestamp in seconds
    pub fn expiration(&self) -> u64 {
        self.exp
    }

    pub fn expired(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("Expect system time to be greater than UNIX_EPOCH")
            .as_secs();
        self.exp < now
    }

    pub fn data(&self) -> &T {
        &self.data
    }
}

/// Opaque token with payload data.
#[derive(Debug)]
pub struct Token<T> {
    pub token: String,
    pub payload: TokenPayload<T>,
}

impl<T> fmt::Display for Token<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.token)
    }
}

impl<T> From<Token<T>> for String {
    fn from(tk: Token<T>) -> Self {
        tk.token
    }
}

#[derive(Debug)]
pub enum TokenEncryptionError {
    /// A invalid token
    ///
    /// When a token may not have a valid JWT shape, encoding or payload.
    InvalidToken,

    /// Invalid algorithm.
    InvalidAlgorithm,

    /// A expired token with valid signature and payload.
    TokenExpired,

    /// A token with a valid signature and a invalid payload
    InvalidPayload,
}

impl From<jsonwebtoken::errors::Error> for TokenEncryptionError {
    fn from(err: jsonwebtoken::errors::Error) -> Self {
        Self::from(err.into_kind())
    }
}

impl From<jsonwebtoken::errors::ErrorKind> for TokenEncryptionError {
    fn from(err: jsonwebtoken::errors::ErrorKind) -> Self {
        match err {
            jsonwebtoken::errors::ErrorKind::InvalidToken => Self::InvalidToken,
            jsonwebtoken::errors::ErrorKind::InvalidSignature => Self::InvalidPayload,
            jsonwebtoken::errors::ErrorKind::InvalidEcdsaKey => Self::InvalidAlgorithm,
            jsonwebtoken::errors::ErrorKind::InvalidRsaKey(_) => Self::InvalidAlgorithm,
            jsonwebtoken::errors::ErrorKind::RsaFailedSigning => Self::InvalidAlgorithm,
            jsonwebtoken::errors::ErrorKind::InvalidAlgorithmName => Self::InvalidAlgorithm,
            jsonwebtoken::errors::ErrorKind::InvalidKeyFormat => Self::InvalidAlgorithm,
            jsonwebtoken::errors::ErrorKind::MissingRequiredClaim(_) => Self::InvalidPayload,
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => Self::TokenExpired,
            jsonwebtoken::errors::ErrorKind::InvalidIssuer => Self::InvalidPayload,
            jsonwebtoken::errors::ErrorKind::InvalidAudience => Self::InvalidPayload,
            jsonwebtoken::errors::ErrorKind::InvalidSubject => Self::InvalidPayload,
            jsonwebtoken::errors::ErrorKind::ImmatureSignature => Self::InvalidPayload,
            jsonwebtoken::errors::ErrorKind::InvalidAlgorithm => Self::InvalidAlgorithm,
            jsonwebtoken::errors::ErrorKind::MissingAlgorithm => Self::InvalidAlgorithm,
            jsonwebtoken::errors::ErrorKind::Base64(_) => Self::InvalidToken,
            jsonwebtoken::errors::ErrorKind::Json(_) => Self::InvalidToken,
            jsonwebtoken::errors::ErrorKind::Utf8(_) => Self::InvalidToken,
            jsonwebtoken::errors::ErrorKind::Crypto(_) => Self::InvalidToken,
            _ => Self::InvalidToken,
        }
    }
}
