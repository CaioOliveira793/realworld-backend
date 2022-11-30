pub mod security {
    use argon2::{Algorithm, Argon2, Params, PasswordHasher, Version};
    use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation};
    use serde::{de::DeserializeOwned, Serialize};

    use crate::domain::datatype::security::{
        PasswordHash, PasswordHashAlgorithm, PasswordHashError, TokenEncryptionError, TokenIssuer,
        TokenPayload,
    };
    use crate::domain::service::{PasswordHashService, TokenEncryptionService};

    pub struct Argon2HashService(Argon2<'static>);

    impl Argon2HashService {
        pub const ALGORITHM: PasswordHashAlgorithm = PasswordHashAlgorithm::Argon2id;
        pub const VERSION: u32 = Version::V0x13 as u32;

        pub fn new() -> Self {
            Self(Argon2::new(
                Algorithm::Argon2id,
                Version::V0x13,
                Params::new(
                    Params::DEFAULT_M_COST,
                    Params::DEFAULT_T_COST,
                    Params::DEFAULT_P_COST,
                    Some(Params::DEFAULT_OUTPUT_LEN),
                )
                .expect("Expect valid default Argon2 params"),
            ))
        }
    }

    impl PasswordHashService for Argon2HashService {
        fn hash_password(&self, pwd: &str) -> Result<PasswordHash, PasswordHashError> {
            let salt = password_hash::SaltString::generate(&mut rand_core::OsRng);

            let mut buf = [0; Params::DEFAULT_OUTPUT_LEN];
            self.0
                .hash_password_into(pwd.as_bytes(), salt.as_bytes(), &mut buf)?;

            let hash = password_hash::Output::new(&buf)?;

            Ok(PasswordHash::new(
                Self::ALGORITHM,
                Some(Self::VERSION),
                self.0.params().try_into()?,
                Some(salt),
                Some(hash),
            ))
        }

        fn verify_password(&self, pwd: &str, hash: &PasswordHash) -> Result<(), PasswordHashError> {
            if let (Some(salt), Some(expected_output)) = (&hash.salt(), &hash.hash()) {
                let computed_hash = self.0.hash_password_customized(
                    pwd.as_bytes(),
                    Some(password_hash::Ident::from(hash.algorithm())),
                    hash.version(),
                    Params::try_from(hash)?,
                    salt.as_salt(),
                )?;

                if let Some(computed_output) = &computed_hash.hash {
                    if expected_output == computed_output {
                        return Ok(());
                    }
                }
            }

            Err(PasswordHashError::InvalidPassword)
        }
    }

    struct JWTEncryptionService {
        secret: String,
        header: Header,
        encoding_key: EncodingKey,
        decoding_key: DecodingKey,
        validation: Validation,
    }

    impl JWTEncryptionService {
        pub fn new(secret: String) -> Self {
            let mut validation = Validation::new(jsonwebtoken::Algorithm::HS256);
            validation.set_required_spec_claims(&["exp", "iss", "sub"]);
            validation.set_issuer(&[TokenIssuer::as_str()]);
            validation.leeway = 60;
            validation.validate_exp = true;
            validation.validate_nbf = false;

            Self {
                encoding_key: EncodingKey::from_secret(secret.as_ref()),
                decoding_key: DecodingKey::from_secret(secret.as_ref()),
                secret,
                header: Header::new(jsonwebtoken::Algorithm::HS256),
                validation,
            }
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
            let token_data = jsonwebtoken::decode::<TokenPayload<T>>(
                token,
                &self.decoding_key,
                &self.validation,
            )?;
            Ok(token_data.claims)
        }
    }
}
