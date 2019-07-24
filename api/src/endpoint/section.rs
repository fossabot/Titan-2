use crate::{
    controller::{
        ExternalLockSection,
        InsertSection,
        LockSection,
        Section,
        Thread,
        UpdateSection,
        User,
    },
    endpoint::helpers::RocketResult,
    DataDB,
};
use rocket::{delete, http::Status, patch, post, response::status::Created};
use rocket_contrib::json::Json;
use std::{
    convert::TryFrom,
    time::{SystemTime, UNIX_EPOCH},
};

/// How long are section locks able to be held for, guaranteed?
///
/// Currently 10 minutes,
/// although this is an implementation detail and should not be relied upon.
const LOCK_DURATION_SECONDS: i64 = 10 * 60;

generic_all!(Section);
generic_get!(Section);

/// Create a `Section`.
#[post("/", data = "<data>")]
pub fn post(
    conn: DataDB,
    user: User,
    data: Json<InsertSection>,
) -> RocketResult<Created<Json<Section>>> {
    if !user.can_modify_thread(&conn, data.in_thread_id) {
        return Err(Status::Unauthorized);
    }

    let ret_val = created!(Section::create(&conn, &data));

    Thread::find_id(&conn, data.in_thread_id)
        .expect("thread not found")
        .update_on_reddit(&conn)
        .expect("error posting on Reddit");

    ret_val
}

/// We need to define a type discriminant to allow Rocket to discern between
/// an update on the lock and an update on everything else.
/// Rather than checking the existence of a field,
/// we can rely on Serde to do that for us.
/// As a bonus, it's future proof if we need to add additional fields.
#[derive(serde::Deserialize)]
#[serde(untagged)]
pub enum UpdateSectionDiscriminant {
    LockSection(ExternalLockSection),
    UpdateSection(UpdateSection),
}

/// Discriminate between the two types,
/// calling the appropriate method as necessary.
#[patch("/<id>", data = "<data>")]
pub fn patch(
    conn: DataDB,
    user: User,
    id: i32,
    data: Json<UpdateSectionDiscriminant>,
) -> RocketResult<Json<Section>> {
    use UpdateSectionDiscriminant::{LockSection, UpdateSection};

    match data.into_inner() {
        LockSection(data) => set_lock(conn, user, id, data),
        UpdateSection(data) => update_fields(conn, user, id, data),
    }
}

/// Set the lock on a `Section`,
/// preventing any other `User`s from updating any fields.
fn set_lock(
    conn: DataDB,
    user: User,
    id: i32,
    data: ExternalLockSection,
) -> RocketResult<Json<Section>> {
    let section = match Section::find_id(&conn, id) {
        Ok(section) => section,
        Err(_) => return Err(Status::NotFound),
    };

    // Ensure the user possesses the authority to modify the lock if able to.
    if !user.can_modify_thread(&conn, section.in_thread_id) {
        return Err(Status::Unauthorized);
    }

    let current_unix_timestamp = i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    )
    .expect("conversion failed");

    // (1) Let the user assign the (currently null) lock to themselves.
    // (2) Let the user revoke their own lock.
    // (3) Let the user renew their own lock.
    // (4) A user holds a lock, but it has been held beyond the specified minimum duration.
    //     Allow the requesting user to possess the lock.
    if (section.lock_held_by_user_id.is_none() && data.lock_held_by_user_id == Some(user.id))
        || (section.lock_held_by_user_id == Some(user.id) && data.lock_held_by_user_id.is_none())
        || (section.lock_held_by_user_id == Some(user.id)
            && data.lock_held_by_user_id == Some(user.id))
        || (section.lock_assigned_at_utc + LOCK_DURATION_SECONDS <= current_unix_timestamp)
    {
        json_result!(Section::set_lock(
            &conn,
            id,
            &LockSection {
                lock_held_by_user_id: data.lock_held_by_user_id,
                lock_assigned_at_utc: current_unix_timestamp,
            }
        ))
    } else {
        // The user isn't setting the lock to themselves,
        // or they possess the lock and are trying to set it to another user.
        Err(Status::Forbidden)
    }
}

/// Update any fields aside from the lock.
fn update_fields(
    conn: DataDB,
    user: User,
    id: i32,
    data: UpdateSection,
) -> RocketResult<Json<Section>> {
    let section = match Section::find_id(&conn, id) {
        Ok(section) => section,
        Err(_) => return Err(Status::NotFound),
    };

    if !user.can_modify_thread(&conn, section.in_thread_id) {
        return Err(Status::Unauthorized);
    }

    let ret_val = json_result!(Section::update(&conn, id, &data));

    Thread::find_id(&conn, section.in_thread_id)
        .expect("thread not found")
        .update_on_reddit(&conn)
        .expect("error updating on Reddit");

    ret_val
}

/// Delete a `Section` and any references to its ID.
#[delete("/<id>")]
pub fn delete(conn: DataDB, user: User, id: i32) -> RocketResult<Status> {
    let section = match Section::find_id(&conn, id) {
        Ok(section) => section,
        Err(_) => return Err(Status::NotFound),
    };

    if !user.can_modify_thread(&conn, section.in_thread_id) {
        return Err(Status::Unauthorized);
    }

    let ret_val = no_content!(Section::delete(&conn, id));

    Thread::find_id(&conn, section.in_thread_id)
        .expect("thread not found")
        .update_on_reddit(&conn)
        .expect("error updating on Reddit");

    ret_val
}
