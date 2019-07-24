#[cfg(test)]
use crate::guid;
use crate::{
    controller::{Claim, InsertUser, User},
    encryption::encrypt,
    DataDB,
};
use lazy_static::lazy_static;
use reddit::Reddit;
use request::Url;
use reqwest as request;
#[cfg(test)]
use rocket::uri;
use rocket::{
    get,
    http::{Cookie, Cookies, RawStr},
    response::Redirect,
};
#[cfg(test)]
use std::time::{Duration, SystemTime};
use std::{convert::TryFrom, error::Error, time::UNIX_EPOCH};

lazy_static! {
    pub static ref REDDIT: Reddit<'static> = Reddit::builder()
        .redirect_uri(dotenv!("REDDIT_REDIRECT_URI"))
        .user_agent(dotenv!("REDDIT_USER_AGENT"))
        .client_id(dotenv!("REDDIT_CLIENT_ID"))
        .secret(dotenv!("REDDIT_SECRET"))
        .permanent(true)
        .scopes({
            use reddit::Scope::*;
            &[
                Account,  // Find language
                Identity, // Find username
                Submit,   // Submit threads
                Edit,     // Update threads
                ModPosts, // (Moderators) Approve a post so it's visible
                ModFlair, // (Moderators) Add/remove/edit a flair on the submission
            ]
        })
        .build()
        .unwrap();
}

/// Endpoint that redirects the user to Reddit,
/// requesting to provided permissions.
///
/// In testing, this endpoint immediately forwards to `callback()`
/// in order to avoid any user input or external requests.
#[get("/?<callback>")]
pub fn oauth(
    conn: DataDB,
    mut cookies: Cookies<'_>,
    callback: &RawStr,
) -> Result<Redirect, Box<dyn Error>> {
    let callback = callback.to_string();

    // If the user has previously authenticated with Enceladus on this device,
    // they should have a cookie set with their user ID.
    // Let's read that and avoid sending the user to re-authenticate.
    if let Some(user_id) = cookies.get_private("user_id") {
        let user = User::find_id(&conn, user_id.value().parse()?)?;

        // Give the user a token that should be used in the future.
        let token = Claim::new(user.id).encode()?;

        // Attach additional querystring parameters to the provided callback.
        let callback = Url::parse_with_params(
            &callback,
            &[
                ("user_id", &user.id.to_string()),
                ("username", &user.reddit_username),
                ("lang", &user.lang),
                ("token", &token),
            ],
        )?;

        return Ok(Redirect::to(callback.to_string()));
    }

    // We're testing; let's not bother with actual authentication.
    // Instead, pretend it succeeded and immediately redirect to the callback.
    #[cfg(test)]
    return Ok(Redirect::to(uri!(
        "/oauth",
        callback: code = guid(),
        state = &callback
    )));

    // Send the user off to Reddit for authentication
    #[cfg(not(test))]
    Ok(Redirect::to(REDDIT.get_auth_url(&callback)?))
}

/// Handle the OAuth response from Reddit.
///
/// Here, we are provided with a refresh token in response to the external OAuth request,
/// and use that token to obtain the user's username and preferred language.
/// All of these values are then used to construct a User
/// which is inserted into the database.
/// We then use the ID returned from the database insertion to generate a
/// [JSON Web Token](https://jwt.io/), which is the user's bearer token
/// that should be provided in the header of each request.
/// Finally, we call the callback URL originally provided,
/// with the additional queryparams of `user_id`, `username`, `lang`, and `token`.
///
/// Please note that `is_global_admin`, along with any specific subreddit values,
/// are **not** initialized, but rather use default values.
/// In the future, there may be an endpoint in the `/v1/user` namespace (or similar)
/// that will automatically perform verification of moderator status
/// (or anything else deemed appropriate).
/// Until that time,
/// these fields must be managed manually,
/// typically by contacting the database operator.
#[cfg_attr(test, allow(unused_variables))]
#[get("/callback?<code>&<state>")]
pub fn callback(
    conn: DataDB,
    mut cookies: Cookies<'_>,
    code: String,
    state: String,
) -> Result<Redirect, Box<dyn Error>> {
    let mut reddit_user;
    let lang;
    let username;

    #[cfg(test)]
    {
        reddit_user = reddit::User::builder()
            .reddit_instance(&REDDIT)
            .refresh_token(guid())
            .access_token(guid())
            .expires_at(SystemTime::now() + Duration::from_secs(3600))
            .build()
            .unwrap();
        lang = guid()[0..2].to_owned();
        username = guid();
    }
    #[cfg(not(test))]
    {
        reddit_user = REDDIT.obtain_refresh_token(&code)?;
        username = reddit_user.username()?;
        lang = reddit_user.lang()?;
    }

    // Insert the user into our database.
    let user = User::create(
        &conn,
        &InsertUser {
            reddit_username: username.to_owned(),
            lang: lang.to_owned(),
            refresh_token: encrypt(reddit_user.refresh_token()),
            is_global_admin: false,
            spacex__is_host: false,
            spacex__is_mod: false,
            spacex__is_slack_member: false,
            access_token: encrypt(reddit_user.access_token()),
            access_token_expires_at_utc: i64::try_from(
                reddit_user
                    .expires_at()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            )
            .expect("conversion failed"),
        },
    )?;

    // Add a cookie so we don't have to re-authetnicate when the user visits again.
    cookies.add_private(
        Cookie::build("user_id", user.id.to_string())
            .permanent()
            .finish(),
    );

    // Give the user a token that should be used in the future.
    let token = Claim::new(user.id).encode()?;

    // Attach additional querystring parameters to the provided callback.
    let callback = Url::parse_with_params(
        &state, // The state doubles as our callback.
        &[
            ("user_id", &user.id.to_string()),
            ("username", &username),
            ("lang", &lang),
            ("token", &token),
        ],
    )?;

    Ok(Redirect::to(callback.to_string()))
}
