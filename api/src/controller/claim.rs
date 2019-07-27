use dotenv_codegen::dotenv;
use jsonwebtoken as jwt;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

static HEADER: Lazy<jwt::Header> = Lazy::new(jwt::Header::default);
static VALIDATION: Lazy<jwt::Validation> = Lazy::new(|| jwt::Validation {
    validate_iat: true,
    validate_exp: false,
    ..jwt::Validation::default()
});
static ROCKET_SECRET_KEY: Lazy<&[u8]> = Lazy::new(|| dotenv!("ROCKET_SECRET_KEY").as_bytes());

/// This represents the body ("claim") of the JWT used for authorization.
/// The `user_id` matches with the ID of a `User` object in the database,
/// while `iat` is the UTC timestamp the token was issued at.
#[derive(Serialize, Deserialize, Debug)]
pub struct Claim {
    user_id: i32,
    iat:     u64,
}

impl Claim {
    /// Create a new `Claim` object with the provided `user_id`.
    /// The `iat` field is automatically generated.
    pub fn new(user_id: i32) -> Self {
        Self {
            user_id,
            iat: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }

    /// Convert the existing `struct` into a valid JWT.
    pub fn encode(&self) -> Result<String, jsonwebtoken::errors::Error> {
        jwt::encode(&HEADER, self, &ROCKET_SECRET_KEY)
    }

    /// Obtain the `user_id` field of a JWT passed as a parameter.
    pub fn get_user_id(token: &str) -> Result<i32, jsonwebtoken::errors::Error> {
        Ok(jwt::decode::<Self>(token, &ROCKET_SECRET_KEY, &VALIDATION)?
            .claims
            .user_id)
    }
}
