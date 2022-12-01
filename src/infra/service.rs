use argon2::{Algorithm, Argon2, Params, Version};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation};
use serde::{de::DeserializeOwned, Serialize};

use crate::domain::datatype::security::{
    OutputHash, PasswordHash, PasswordHashAlgorithm, PasswordHashError, SaltString,
    TokenEncryptionError, TokenIssuer, TokenPayload,
};
use crate::domain::service::{PasswordHashService, TokenEncryptionService};

pub struct Argon2HashService(Argon2<'static>);

impl Argon2HashService {
    pub const ALGORITHM: PasswordHashAlgorithm = PasswordHashAlgorithm::Argon2id;
    pub const VERSION: u32 = Version::V0x13 as u32;
    pub const HASH_OUTPUT_LENGTH: usize = Params::DEFAULT_OUTPUT_LEN;

    pub fn new() -> Self {
        Self(Argon2::new(
            Algorithm::Argon2id,
            Version::V0x13,
            Params::new(
                Params::DEFAULT_M_COST,
                Params::DEFAULT_T_COST,
                Params::DEFAULT_P_COST,
                Some(Self::HASH_OUTPUT_LENGTH),
            )
            .expect("Expect valid default Argon2 params"),
        ))
    }
}

impl PasswordHashService for Argon2HashService {
    fn hash_password(&self, pwd: &str) -> Result<PasswordHash, PasswordHashError> {
        let salt = SaltString::generate(&mut rand_core::OsRng);

        let mut buf = [0; Self::HASH_OUTPUT_LENGTH];
        self.0
            .hash_password_into(pwd.as_bytes(), salt.as_bytes(), &mut buf)?;

        let hash = OutputHash::new(&buf)?;

        Ok(PasswordHash::new(
            Self::ALGORITHM,
            Some(Self::VERSION),
            self.0.params().try_into()?,
            Some(salt),
            Some(hash),
        ))
    }

    fn verify_password(&self, pwd: &str, hash: &PasswordHash) -> Result<(), PasswordHashError> {
        if let (Some(salt), Some(expected_output)) = (hash.salt(), hash.hash()) {
            let argon2 = Argon2::new(
                Algorithm::try_from(hash.algorithm().clone())?,
                Version::try_from(hash.version().unwrap_or_default())?,
                Params::try_from(hash)?,
            );

            let mut buf = [0; Self::HASH_OUTPUT_LENGTH];
            argon2.hash_password_into(pwd.as_bytes(), salt.as_bytes(), &mut buf)?;
            let computed_output = OutputHash::new(&buf)?;

            if *expected_output == computed_output {
                return Ok(());
            }
        }

        Err(PasswordHashError::InvalidPassword)
    }
}

#[cfg(test)]
mod argon2_hash_service_test {
    use std::str::FromStr;

    use pretty_assertions::assert_eq;

    use super::Argon2HashService;
    use crate::domain::{datatype::security::PasswordHash, service::PasswordHashService};

    #[test]
    fn hash_serialize_and_verify_password() {
        let argon2 = Argon2HashService::new();

        let pwds = [
            "super_secret",
            "12345678",
            "onw*(*#028]][2389nfwCSOEN",
            "no",
        ];

        for pwd in pwds {
            let hash = argon2
                .hash_password(pwd)
                .expect("Expect to hash the password");

            let deserialized = PasswordHash::from_str(&hash.to_string())
                .expect("Expect to deserialize the password");
            assert_eq!(deserialized, hash);

            assert_eq!(argon2.verify_password(pwd, &deserialized), Ok(()));
        }
    }
}

pub struct JWTEncryptionService {
    header: Header,
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    validation: Validation,
}

impl JWTEncryptionService {
    pub fn new(secret: &[u8]) -> Self {
        let mut validation = Validation::new(jsonwebtoken::Algorithm::HS256);
        validation.set_required_spec_claims(&["exp", "iss", "sub"]);
        validation.set_issuer(&[TokenIssuer::as_str()]);
        validation.leeway = 60;
        validation.validate_exp = true;
        validation.validate_nbf = false;

        Self {
            encoding_key: EncodingKey::from_secret(secret),
            decoding_key: DecodingKey::from_secret(secret),
            header: Header::new(jsonwebtoken::Algorithm::HS256),
            validation,
        }
    }

    pub fn from_config() -> Self {
        Self::new(crate::config::env_var::get().token_key.as_ref())
    }
}

impl TokenEncryptionService for JWTEncryptionService {
    fn issue_token<T>(&self, payload: &TokenPayload<T>) -> Result<String, TokenEncryptionError>
    where
        T: Serialize,
    {
        let token = jsonwebtoken::encode(&self.header, payload, &self.encoding_key)?;
        Ok(token)
    }

    fn verify_token<T>(&self, token: &str) -> Result<TokenPayload<T>, TokenEncryptionError>
    where
        T: DeserializeOwned,
    {
        let token_data = jsonwebtoken::decode(token, &self.decoding_key, &self.validation)?;
        Ok(token_data.claims)
    }
}

#[cfg(test)]
mod jwt_encryption_service_test {
    use std::{cmp, fmt, time::Duration};

    use serde::{de::DeserializeOwned, Deserialize, Serialize};
    use uuid::Uuid;

    use super::JWTEncryptionService;
    use crate::domain::{
        datatype::security::{TokenPayload, TokenSubject},
        service::TokenEncryptionService,
    };

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct RolesPayload {
        roles: Vec<String>,
    }

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct ActionsPayload {
        actions: Vec<String>,
    }

    #[test]
    fn issue_token_and_verify() {
        fn issue_and_verify<T: Serialize + DeserializeOwned + fmt::Debug + cmp::PartialEq>(
            jwt: &JWTEncryptionService,
            payload: TokenPayload<T>,
        ) {
            let token = jwt
                .issue_token(&payload)
                .expect("Expect to issue the token");

            let parsed_payload = jwt
                .verify_token(&token)
                .expect("Expect to verify the token");

            assert_eq!(parsed_payload, payload);
        }

        let jwt = JWTEncryptionService::new("my_secret".as_bytes());

        issue_and_verify(
            &jwt,
            TokenPayload::new(Duration::from_secs(10), TokenSubject::Public, ()),
        );

        issue_and_verify(
            &jwt,
            TokenPayload::new(
                Duration::from_secs(10),
                TokenSubject::User(Uuid::new_v4()),
                RolesPayload {
                    roles: vec!["admin".into()],
                },
            ),
        );

        issue_and_verify(
            &jwt,
            TokenPayload::new(
                Duration::from_secs(10),
                TokenSubject::Public,
                RolesPayload { roles: vec![] },
            ),
        );
    }
}
