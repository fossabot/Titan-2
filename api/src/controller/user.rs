#![allow(non_snake_case)]

use super::{Claim, Thread, USER_CACHE_SIZE};
use crate::{
    encryption::{decrypt, encrypt},
    endpoint::oauth::REDDIT,
    schema::user::{self, dsl::*},
    websocket::*,
    DataDB,
    Database,
};
use enceladus_macros::generate_structs;
use lazy_static::lazy_static;
use lru_cache::LruCache;
use parking_lot::Mutex;
use rocket::{
    http::Status,
    request::{self, FromRequest, Request},
    Outcome,
};
use rocket_contrib::databases::diesel::{ExpressionMethods, QueryDsl, QueryResult, RunQueryDsl};
use std::time::{Duration, UNIX_EPOCH};
#[cfg(debug_assertions)]
use {rocket_contrib::json::Json, serde::Deserialize, serde_json::json};

lazy_static! {
    /// A global cache, containing a mapping of IDs to their respective `Event`.
    ///
    /// The cache is protected by a `Mutex`,
    /// ensuring there is only ever at most one writer at a time.
    /// Note that even when reading,
    /// there must be a lock on mutability,
    /// as the `LruCache` must be able to update itself.
    ///
    /// To read from the cache,
    /// you'll want to call `CACHE.lock()` before performing normal operations.
    /// ```
    static ref CACHE: Mutex<LruCache<i32, User>> = Mutex::new(LruCache::new(USER_CACHE_SIZE));
}

generate_structs! {
    User("user") {
        auto id: i32,
        readonly reddit_username: String,
        lang: String = "en",
        private refresh_token: Vec<u8>,
        is_global_admin: bool = false,
        spacex__is_admin: bool = false,
        spacex__is_mod: bool = false,
        spacex__is_slack_member: bool = false,
        private access_token: Vec<u8>,
        private access_token_expires_at_utc: i64,
    }
}

// TODO make these macros!

/// This struct is necessary to perform the requisite encryption
/// of the refresh and access tokens.
/// It is otherwise identical to `UpdateUser`.
#[cfg(debug_assertions)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalUpdateUser {
    pub lang: Option<String>,
    pub refresh_token: Option<String>,
    pub is_global_admin: Option<bool>,
    pub spacex__is_admin: Option<bool>,
    pub spacex__is_mod: Option<bool>,
    pub spacex__is_slack_member: Option<bool>,
    pub access_token: Option<String>,
    pub access_token_expires_at_utc: Option<i64>,
}

#[cfg(debug_assertions)]
impl Into<UpdateUser> for Json<ExternalUpdateUser> {
    /// Convert the `Json<ExternalUpdateUser>` from the endpoint
    /// into an `UpdateUser` for consumption by the controller.
    ///
    /// The sole purpose of this conversion is to encrypt the
    /// refresh and access tokens where necessary.
    #[inline]
    fn into(self) -> UpdateUser {
        UpdateUser {
            lang: self.lang.clone(),
            refresh_token: self.refresh_token.clone().map(|s| encrypt(&s)),
            is_global_admin: self.is_global_admin,
            spacex__is_admin: self.spacex__is_admin,
            spacex__is_mod: self.spacex__is_mod,
            spacex__is_slack_member: self.spacex__is_slack_member,
            access_token: self.access_token.clone().map(|s| encrypt(&s)),
            access_token_expires_at_utc: self.access_token_expires_at_utc,
        }
    }
}

/// Helper function for serde to have a default value when deserializing.
#[cfg(debug_assertions)]
#[inline(always)]
fn en() -> String {
    "en".into()
}

/// Helper function for serde to have a default value when deserializing.
#[cfg(debug_assertions)]
#[inline(always)]
const fn falsey() -> bool {
    false
}

/// This struct is necessary to perform the requisite encryption
/// of the refresh and access tokens.
/// It is otherwise identical to `InsertUser`.
#[cfg(debug_assertions)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalInsertUser {
    pub reddit_username: String,
    #[serde(default = "en")]
    pub lang: String,
    pub refresh_token: String,
    #[serde(default = "falsey")]
    pub is_global_admin: bool,
    #[serde(default = "falsey")]
    pub spacex__is_admin: bool,
    #[serde(default = "falsey")]
    pub spacex__is_mod: bool,
    #[serde(default = "falsey")]
    pub spacex__is_slack_member: bool,
    pub access_token: String,
    pub access_token_expires_at_utc: i64,
}

#[cfg(debug_assertions)]
impl Into<InsertUser> for Json<ExternalInsertUser> {
    /// Convert the `Json<ExternalInsertUser>` from the endpoint
    /// into an `InsertUser` for consumption by the controller.
    ///
    /// The sole purpose of this conversion is to encrypt the
    /// refresh and access tokens where necessary.
    #[inline]
    fn into(self) -> InsertUser {
        InsertUser {
            reddit_username: self.reddit_username.clone(),
            lang: self.lang.clone(),
            refresh_token: encrypt(&self.refresh_token),
            is_global_admin: self.is_global_admin,
            spacex__is_admin: self.spacex__is_admin,
            spacex__is_mod: self.spacex__is_mod,
            spacex__is_slack_member: self.spacex__is_slack_member,
            access_token: encrypt(&self.access_token),
            access_token_expires_at_utc: self.access_token_expires_at_utc,
        }
    }
}

impl User {
    /// Check if the user is a moderator of a given subreddit.
    ///
    /// If the subreddit is not known, returns `false`.
    #[inline]
    pub fn is_moderator_of(&self, subreddit: Option<&str>) -> bool {
        match subreddit {
            Some("spacex") => self.spacex__is_mod,
            _ => false,
        }
    }

    /// Check if the user is an admin of a given subreddit.
    ///
    /// If the subreddit is not known, returns `false`.
    #[inline]
    pub fn is_admin_of(&self, subreddit: Option<&str>) -> bool {
        match subreddit {
            Some("spacex") => self.spacex__is_admin,
            _ => false,
        }
    }

    /// Is the provided user able to modify data (including sections and events)
    /// on the indicated thread?
    ///
    /// Authentication levels (from low to high):
    ///
    /// - None
    /// - Logged in (everyday user)
    /// - Thread author
    /// - Subreddit admin
    /// - Global admin
    ///
    /// This function verifies that a user is, at a minimum, the thread author.
    #[inline]
    pub fn can_modify_thread(&self, conn: &DataDB, thread_id: i32) -> bool {
        // Global admins can change anything.
        if self.is_global_admin {
            return true;
        }

        let thread = {
            let thread = Thread::find_id(conn, thread_id);

            // The thread we want to add the event to doesn't exist.
            if thread.is_err() {
                return false;
            }

            thread.unwrap()
        };

        // The user is a local admin.
        if self.is_admin_of(thread.subreddit.as_ref().map(String::as_str)) {
            return true;
        }

        // The user is the thread creator.
        thread.created_by_user_id == self.id
    }

    /// When performing any request to Reddit,
    /// we need to send an access token to authenticate ourselves.
    /// These tokens must be refreshed every hour (currently; that's subject to change).
    /// As such, this should be handled automatically wherever possible.
    ///
    /// What we do here is read in the user from the database,
    /// check if their token should be expired (given the previously calculated timestamp).
    /// If it is, let's request a new one from Reddit and store that
    /// (along with its new expiration time) in the database.
    #[inline]
    pub fn update_access_token_if_necessary(
        conn: &Database,
        user_id: i32,
        reddit_user: &mut reddit::User<'_>,
    ) -> QueryResult<Self> {
        let db_user = Self::find_id(conn, user_id)?;
        let current_expires_at = db_user.access_token_expires_at_utc;
        let new_expires_at = reddit_user
            .expires_at()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        if current_expires_at != new_expires_at {
            Self::update(
                conn,
                user_id,
                &UpdateUser {
                    access_token: encrypt(reddit_user.access_token().as_ref()).into(),
                    access_token_expires_at_utc: new_expires_at.into(),
                    ..UpdateUser::default()
                },
            )
        } else {
            Ok(db_user)
        }
    }

    /// Find all `User`s in the database.
    ///
    /// Does _not_ use cache (reading or writing),
    /// so as to avoid storing values rarely accessed.
    #[inline]
    pub fn find_all(conn: &Database) -> QueryResult<Vec<Self>> {
        user.load(conn)
    }

    /// Find a specific `User` given its ID.
    ///
    /// Internally uses a cache to limit database accesses.
    #[inline]
    pub fn find_id(conn: &Database, user_id: i32) -> QueryResult<Self> {
        let mut cache = CACHE.lock();
        if cache.contains_key(&user_id) {
            Ok(cache.get_mut(&user_id).unwrap().clone())
        } else {
            let result: Self = user.find(user_id).first(conn)?;
            cache.insert(user_id, result.clone());
            Ok(result)
        }
    }

    /// Create a `User` given the data.
    ///
    /// The inserted row is added to the global cache and returned.
    #[inline]
    pub fn create(conn: &Database, data: &InsertUser) -> QueryResult<Self> {
        let result: Self = diesel::insert_into(user).values(data).get_result(conn)?;
        CACHE.lock().insert(result.id, result.clone());

        let _ = Message {
            room:      Room::User,
            action:    Action::Create,
            data_type: DataType::User,
            data:      &result,
        }
        .send();

        Ok(result)
    }

    /// Update a `User` given an ID and the data to update.
    ///
    /// The entry is updated in the database, added to cache, and returned.
    #[inline]
    pub fn update(conn: &Database, user_id: i32, data: &UpdateUser) -> QueryResult<Self> {
        let result: Self = diesel::update(user)
            .filter(id.eq(user_id))
            .set(data)
            .get_result(conn)?;
        CACHE.lock().insert(result.id, result.clone());

        let _ = Message {
            room:      Room::User,
            action:    Action::Update,
            data_type: DataType::User,
            data:      &Update::new(user_id, data),
        }
        .send();

        Ok(result)
    }

    /// Delete a `User` given its ID.
    ///
    /// Removes the entry from cache and returns the number of rows deleted (should be `1`).
    #[cfg(debug_assertions)]
    #[inline]
    pub fn delete(conn: &Database, user_id: i32) -> QueryResult<usize> {
        CACHE.lock().remove(&user_id);

        let _ = Message {
            room:      Room::User,
            action:    Action::Delete,
            data_type: DataType::User,
            data:      &json!({ "id": user_id }),
        }
        .send();

        diesel::delete(user).filter(id.eq(user_id)).execute(conn)
    }
}

impl<'a, 'r> FromRequest<'a, 'r> for User {
    type Error = &'a str;

    /// Create a request guard requiring a user to be authorized with a previously issued JWT.
    /// If the user is not found or the `Authorization` header is malformed/incorrect,
    /// don't allow the client to continue to the rest of the request.
    #[inline]
    fn from_request(request: &'a Request<'r>) -> request::Outcome<Self, Self::Error> {
        let header = request.headers().get_one("Authorization");
        if header.is_none() {
            return Outcome::Failure((
                Status::Unauthorized,
                r#"Expected "Authorization" header to be present"#,
            ));
        }

        let header_contents = header.unwrap();
        if !header_contents.starts_with("bearer ") && !header_contents.starts_with("Bearer ") {
            return Outcome::Failure((
                Status::BadRequest,
                r#"Expected "Authorization" header to begin with "bearer " or "Bearer ""#,
            ));
        }

        let user_id = Claim::get_user_id(&header_contents[7..]);
        if user_id.is_err() {
            return Outcome::Failure((
                Status::BadRequest,
                r#""Authorization" header cannot be decoded"#,
            ));
        }

        let database: DataDB = request
            .guard()
            .succeeded()
            .expect("Unable to access database");

        match Self::find_id(&database, user_id.unwrap()) {
            Ok(authenticated_user) => Outcome::Success(authenticated_user),
            Err(_) => Outcome::Failure((Status::BadRequest, "Unable to find user")),
        }
    }
}

impl<'a> Into<reddit::User<'a>> for User {
    /// Create a `reddit::User` from a `User`.
    /// Automatically decrypts the refresh and access tokens.
    #[inline]
    fn into(self) -> reddit::User<'a> {
        reddit::User::builder()
            .reddit_instance(&REDDIT)
            .refresh_token(decrypt(&*self.refresh_token))
            .access_token(decrypt(&*self.access_token))
            .expires_at(UNIX_EPOCH + Duration::from_secs(self.access_token_expires_at_utc as u64))
            .build()
            .unwrap()
    }
}
