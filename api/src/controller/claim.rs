use jsonwebtoken as jwt;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

lazy_static! {
    static ref HEADER: jwt::Header = jwt::Header::default();
    static ref VALIDATION: jwt::Validation = jwt::Validation {
        validate_iat: true,
        validate_exp: false,
        ..jwt::Validation::default()
    };
}

const ROCKET_SECRET_KEY: &[u8] = dotenv!("ROCKET_SECRET_KEY").as_bytes();

/// This represents the body ("claim") of the JWT used for authorization.
/// The `user_id` matches with the ID of a `User` object in the database,
/// while `iat` is the UTC timestamp the token was issued at.
#[derive(Serialize, Deserialize)]
pub struct Claim {
    user_id: i32,
    iat:     u64,
}

impl Claim {
    /// Create a new `Claim` object with the provided `user_id`.
    /// The `iat` field is automatically generated.
    #[inline]
    pub fn new(user_id: i32) -> Claim {
        Claim {
            user_id,
            iat: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }

    /// Convert the existing `struct` into a valid JWT.
    #[inline]
    pub fn encode(&self) -> Result<String, jsonwebtoken::errors::Error> {
        jwt::encode(&HEADER, self, ROCKET_SECRET_KEY)
    }

    /// Obtain the `user_id` field of a JWT passed as a parameter.
    #[inline]
    pub fn get_user_id(token: &str) -> Result<i32, jsonwebtoken::errors::Error> {
        Ok(jwt::decode::<Claim>(token, ROCKET_SECRET_KEY, &VALIDATION)?
            .claims
            .user_id)
    }
}
