use super::datatype::security::{
    PasswordHash, PasswordHashError, Token, TokenEncryptionError, TokenPayload,
};

pub trait PasswordHashService {
    fn hash_password(&self, pwd: &str) -> Result<PasswordHash, PasswordHashError>;
    fn verify_password(&self, pwd: &str, hash: &PasswordHash) -> Result<(), PasswordHashError>;
}

pub trait TokenEncryptionService {
    fn issue_token<T>(&self, payload: &TokenPayload<T>) -> Result<String, TokenEncryptionError>
    where
        T: serde::Serialize;

    fn verify_token<T>(&self, token: &str) -> Result<TokenPayload<T>, TokenEncryptionError>
    where
        T: serde::de::DeserializeOwned;
}

impl<T> Token<T> {
    pub fn new<TS>(payload: TokenPayload<T>, encrypter: &TS) -> Result<Self, TokenEncryptionError>
    where
        TS: TokenEncryptionService,
        T: serde::Serialize,
    {
        let token = encrypter.issue_token(&payload)?;
        Ok(Self { token, payload })
    }

    pub fn verify<TS>(token: String, encrypter: &TS) -> Result<Self, TokenEncryptionError>
    where
        TS: TokenEncryptionService,
        T: serde::de::DeserializeOwned,
    {
        let payload = encrypter.verify_token(&token)?;
        Ok(Self { token, payload })
    }
}
