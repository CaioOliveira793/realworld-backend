use argon2::{Algorithm, Argon2, Params, PasswordHasher, Version};

use super::datatype::password::{PasswordHash, PasswordHashAlgorithm, PasswordHashError};

pub trait PasswordHashService {
    fn hash_password(&self, pwd: &str) -> Result<PasswordHash, PasswordHashError>;
    fn verify_password(&self, pwd: &str, hash: &PasswordHash) -> Result<(), PasswordHashError>;
}

pub struct Argon2HashService {
    ctx: Argon2<'static>,
}

impl Argon2HashService {
    pub const ALGORITHM: PasswordHashAlgorithm = PasswordHashAlgorithm::Argon2id;
    pub const VERSION: u32 = Version::V0x13 as u32;

    pub fn new() -> Self {
        Self {
            ctx: Argon2::new(
                Algorithm::Argon2id,
                Version::V0x13,
                Params::new(
                    Params::DEFAULT_M_COST,
                    Params::DEFAULT_T_COST,
                    Params::DEFAULT_P_COST,
                    Some(Params::DEFAULT_OUTPUT_LEN),
                )
                .expect("Expect valid default Argon2 params"),
            ),
        }
    }
}

impl PasswordHashService for Argon2HashService {
    fn hash_password(&self, pwd: &str) -> Result<PasswordHash, PasswordHashError> {
        let salt = password_hash::SaltString::generate(&mut rand_core::OsRng);

        let mut buf = [0; Params::DEFAULT_OUTPUT_LEN];
        self.ctx
            .hash_password_into(pwd.as_bytes(), salt.as_bytes(), &mut buf)?;

        let hash = password_hash::Output::new(&buf)?;

        Ok(PasswordHash::new(
            Self::ALGORITHM,
            Some(Self::VERSION),
            self.ctx.params().try_into()?,
            Some(salt),
            Some(hash),
        ))
    }

    fn verify_password(&self, pwd: &str, hash: &PasswordHash) -> Result<(), PasswordHashError> {
        if let (Some(salt), Some(expected_output)) = (&hash.salt(), &hash.hash()) {
            let computed_hash = self.ctx.hash_password_customized(
                pwd.as_bytes(),
                Some(password_hash::Ident::from(hash.algorithm())),
                hash.version(),
                Params::try_from(hash)?,
                salt.as_salt(),
            )?;

            if let Some(computed_output) = &computed_hash.hash {
                // See notes on `Output` about the use of a constant-time comparison
                if expected_output == computed_output {
                    return Ok(());
                }
            }
        }

        Err(PasswordHashError::InvalidPassword)
    }
}
