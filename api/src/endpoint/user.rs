#![allow(non_snake_case)]

use crate::controller::User;

#[cfg(debug)]
use {
    crate::{
        controller::{Claim, ExternalInsertUser, ExternalUpdateUser},
        endpoint::helpers::RocketResult,
        DataDB,
    },
    rocket::{delete, http::Status, patch, post, response::status::Created},
    rocket_contrib::json::Json,
    serde::Serialize,
    std::convert::From,
};

generic_all!(User);
generic_get!(User);

/// Create a `User`.
#[cfg(debug)]
#[post("/", data = "<data>")]
pub fn post(
    conn: DataDB,
    data: Json<ExternalInsertUser>,
) -> RocketResult<Created<Json<TokenUser>>> {
    created!(User::create(&conn, &data.into()).map(TokenUser::from))
}

/// Update a `User`.
#[cfg(debug)]
#[patch("/<id>", data = "<data>")]
pub fn patch(conn: DataDB, id: i32, data: Json<ExternalUpdateUser>) -> RocketResult<Json<User>> {
    json_result!(User::update(&conn, id, &data.into()))
}

/// Delete a `User`.
#[cfg(debug)]
#[delete("/<id>")]
pub fn delete(conn: DataDB, id: i32) -> RocketResult<Status> {
    no_content!(User::delete(&conn, id))
}

// There's no need to use this elsewhere,
// as this struct exists solely to make testing easier.
#[cfg(debug)]
#[derive(Serialize)]
pub struct TokenUser {
    token: String,
    id: i32,
    reddit_username: String,
    lang: String,
    is_global_admin: bool,
    spacex__is_host: bool,
    spacex__is_mod: bool,
    spacex__is_slack_member: bool,
}

// The encoding will never fail given an integer.
#[allow(clippy::fallible_impl_from)]
#[cfg(debug)]
impl From<User> for TokenUser {
    /// Create a `TokenUser`,
    /// which is a `User` that has an additional `token` field containing a JWT.
    fn from(user: User) -> Self {
        Self {
            token: Claim::new(user.id).encode().unwrap(),
            id: user.id,
            reddit_username: user.reddit_username,
            lang: user.lang,
            is_global_admin: user.is_global_admin,
            spacex__is_host: user.spacex__is_host,
            spacex__is_mod: user.spacex__is_mod,
            spacex__is_slack_member: user.spacex__is_slack_member,
        }
    }
}
