use crate::{
    controller::{Event, InsertEvent, Thread, UpdateEvent, User},
    endpoint::helpers::RocketResult,
    DataDB,
};
use rocket::{delete, http::Status, patch, post, response::status::Created};
use rocket_contrib::json::Json;

generic_all!(Event);
generic_get!(Event);

/// Create an `Event`.
#[post("/", data = "<data>")]
pub fn post(
    conn: DataDB,
    user: User,
    data: Json<InsertEvent>,
) -> RocketResult<Created<Json<Event>>> {
    if !user.can_modify_thread(&conn, data.in_thread_id) {
        return Err(Status::Unauthorized);
    }

    let thread = Thread::find_id(&conn, data.in_thread_id).expect("thread not found");

    // Ensure the provided columns are of the expected types and length.
    if !data.cols.is_array()
        || thread.event_column_headers.len() != data.cols.as_array().unwrap().len()
        || !data
            .cols
            .as_array()
            .unwrap()
            .iter()
            .zip(0..)
            .all(|(val, i)| match thread.space__utc_col_index {
                Some(n) if i == n => val.is_number(),
                _ => val.is_string(),
            })
    {
        return Err(Status::UnprocessableEntity);
    }

    let ret_val = created!(Event::create(&conn, &data));
    thread
        .update_on_reddit(&conn)
        .expect("error updating on Reddit");
    ret_val
}

/// We need to define a type discriminant to allow Rocket to discern between
/// an update on all columns and an update on a specific column.
///
/// When updating all columns,
/// we're expecting a regular `UpdateEvent` object.
/// When updating a single column,
/// we're expecting an array containing the [key, new value].
#[derive(serde::Deserialize)]
#[serde(untagged)]
pub enum UpdateEventDiscriminant {
    FullEvent(UpdateEvent),
    PartialEvent(Vec<(usize, serde_json::Value)>),
}

/// Discriminate between the two types,
/// calling the `patch_full_event` method as necessary.
#[patch("/<id>", data = "<data>")]
pub fn patch(
    conn: DataDB,
    user: User,
    id: i32,
    data: Json<UpdateEventDiscriminant>,
) -> RocketResult<Json<Event>> {
    use UpdateEventDiscriminant::{FullEvent, PartialEvent};

    match data.into_inner() {
        FullEvent(data) => patch_full_event(conn, user, id, data),
        PartialEvent(data) => {
            let mut event = match Event::find_id(&conn, id) {
                Ok(event) => event,
                Err(_) => return Err(Status::NotFound),
            };

            let event_fields = &mut event.cols;

            for (key, value) in data {
                event_fields[key] = value;
            }

            patch_full_event(
                conn,
                user,
                id,
                UpdateEvent {
                    cols: Some(event.cols),
                    ..UpdateEvent::default()
                },
            )
        }
    }
}

/// Update the `Event` on Reddit and in the database.
pub fn patch_full_event(
    conn: DataDB,
    user: User,
    id: i32,
    data: UpdateEvent,
) -> RocketResult<Json<Event>> {
    let event = match Event::find_id(&conn, id) {
        Ok(event) => event,
        Err(_) => return Err(Status::NotFound),
    };

    if !user.can_modify_thread(&conn, event.in_thread_id) {
        return Err(Status::Unauthorized);
    }

    let ret_val = json_result!(Event::update(&conn, id, &data));

    Thread::find_id(&conn, event.in_thread_id)
        .expect("thread not found")
        .update_on_reddit(&conn)
        .expect("error updating on Reddit");

    ret_val
}

/// Delete an `Event` as well as any references to its ID.
#[delete("/<id>")]
pub fn delete(conn: DataDB, user: User, id: i32) -> RocketResult<Status> {
    let event = match Event::find_id(&conn, id) {
        Ok(event) => event,
        Err(_) => return Err(Status::Unauthorized),
    };

    if !user.can_modify_thread(&conn, event.in_thread_id) {
        return Err(Status::Unauthorized);
    }

    let ret_val = no_content!(Event::delete(&conn, id));

    Thread::find_id(&conn, event.in_thread_id)
        .expect("thread not found")
        .update_on_reddit(&conn)
        .expect("error updating on Reddit");

    ret_val
}
