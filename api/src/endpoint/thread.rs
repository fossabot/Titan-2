use crate::{
    controller::{ExternalInsertThread, Thread, UpdateThread, User},
    endpoint::helpers::RocketResult,
    DataDB,
};
use rocket::{delete, get, http::Status, patch, post, response::status::Created};
use rocket_contrib::json::{Json, JsonValue};
use std::collections::BTreeSet;

generic_all!(Thread);
generic_get!(Thread);

/// Get the `Thread` along with its `Section`s, `Event`s, author, and section locks.
#[get("/<id>/full")]
pub fn get_full(conn: DataDB, id: i32) -> RocketResult<JsonValue> {
    Ok(Thread::find_id_with_foreign_keys(&conn, id)
        .map_err(|e| crate::endpoint::helpers::error_mapper(&e))?
        .into())
}

/// Create a `Thread`.
#[post("/", data = "<data>")]
pub fn post(
    conn: DataDB,
    user: User,
    data: Json<ExternalInsertThread>,
) -> RocketResult<Created<Json<Thread>>> {
    let user_id = user.id;
    let subreddit = &data.subreddit;
    let mut post_id = None;

    if let Some(subreddit) = subreddit {
        let mut user: reddit::User<'_> = user.into();
        post_id = Some(
            user.submit_self_post(subreddit, &data.thread_name, None)
                .expect("error posting to Reddit"),
        );
        User::update_access_token_if_necessary(&conn, user_id, &mut user)
            .expect("could not update access token");
    }

    created!(Thread::create(&conn, &data, user_id, post_id))
}

/// Update a `Thread`.
#[patch("/<id>", data = "<data>")]
pub fn patch(
    conn: DataDB,
    user: User,
    id: i32,
    data: Json<UpdateThread>,
) -> RocketResult<Json<Thread>> {
    if !user.can_modify_thread(&conn, id) {
        return Err(Status::Unauthorized);
    }

    // Restrict changing `.sections_id` to reordering, not adding or removing.
    if data.sections_id.is_some() {
        let current_thread = Thread::find_id(&conn, id).unwrap();

        let current_sections: BTreeSet<_> = current_thread.sections_id.iter().collect();
        let proposed_sections: BTreeSet<_> = data.sections_id.as_ref().unwrap().iter().collect();

        if current_sections != proposed_sections {
            return Err(Status::PreconditionFailed);
        }
    }

    // Restrict changing `.events_id` to reordering, not adding or removing.
    if data.events_id.is_some() {
        let current_thread = Thread::find_id(&conn, id).unwrap();

        let current_events: BTreeSet<_> = current_thread.events_id.iter().collect();
        let proposed_events: BTreeSet<_> = data.events_id.as_ref().unwrap().iter().collect();

        if current_events != proposed_events {
            return Err(Status::PreconditionFailed);
        }
    }

    Thread::find_id(&conn, id)
        .unwrap()
        .update_on_reddit(&conn)
        .unwrap();

    json_result!(Thread::update(&conn, id, &data))
}

/// Approve a `Thread` on Reddit.
/// Does not perform any action in the database,
/// aside from potentially updating a `User`'s access token.
#[patch("/<id>/approve")]
pub fn approve(conn: DataDB, user: User, id: i32) -> RocketResult<Json<()>> {
    let thread = match Thread::find_id(&conn, id) {
        Ok(thread) => {
            if thread.post_id.is_some() {
                thread
            } else {
                return Err(Status::PreconditionFailed);
            }
        }
        Err(_) => return Err(Status::NotFound),
    };

    if !user.is_moderator_of(thread.subreddit.as_ref().map(String::as_str)) {
        return Err(Status::Unauthorized);
    }

    let mut user: reddit::User<'_> = user.into();
    user.approve(&format!("t3_{}", thread.post_id.unwrap()))
        .expect("error approving thread");
    User::update_access_token_if_necessary(&conn, thread.created_by_user_id, &mut user)
        .expect("could not update access token");

    Ok(Json(()))
}

/// Sticky a `Thread` on Reddit.
/// Does not perform any action in the database,
/// aside from potentially updating a `User`'s access token.
#[patch("/<id>/sticky")]
pub fn sticky(conn: DataDB, user: User, id: i32) -> RocketResult<Json<()>> {
    set_sticky(conn, user, id, true)
}

/// Unsticky a `Thread` on Reddit.
/// Does not perform any action in the database,
/// aside from potentially updating a `User`'s access token.
#[patch("/<id>/unsticky")]
pub fn unsticky(conn: DataDB, user: User, id: i32) -> RocketResult<Json<()>> {
    set_sticky(conn, user, id, false)
}

/// Sets whether a `Thread` should be stickied or unstickied on Reddit.
/// Does not perform any action in the database,
/// aside from potentially updating a `User`'s access token.
fn set_sticky(conn: DataDB, user: User, id: i32, state: bool) -> RocketResult<Json<()>> {
    let thread = match Thread::find_id(&conn, id) {
        Ok(thread) => {
            if thread.post_id.is_some() {
                thread
            } else {
                return Err(Status::PreconditionFailed);
            }
        }
        Err(_) => return Err(Status::NotFound),
    };

    if !user.is_moderator_of(thread.subreddit.as_ref().map(String::as_str)) {
        return Err(Status::Unauthorized);
    }

    let mut user: reddit::User<'_> = user.into();
    user.set_sticky(&format!("t3_{}", thread.post_id.unwrap()), state)
        .expect("error stickying/unstickying thread");
    User::update_access_token_if_necessary(&conn, thread.created_by_user_id, &mut user)
        .expect("could not update access token");

    Ok(Json(()))
}

/// Delete a `Thread`.
#[delete("/<id>")]
pub fn delete(conn: DataDB, user: User, id: i32) -> RocketResult<Status> {
    if user.can_modify_thread(&conn, id) {
        no_content!(Thread::delete(&conn, id))
    } else {
        Err(Status::Unauthorized)
    }
}
