use crate::{guid, tests::helpers::*};
use serde_json::{json, Value as Json};

const BASE: &str = "/v1/event";

fn create_event(client: &mut Client<'_>, token: &str, thread_id: i32) -> Json {
    client
        .with_base(BASE)
        .post(
            Some(token),
            json!({
                "cols": [1_500_000_000, guid!(), guid!()],
                "in_thread_id": thread_id,
            }),
        )
        .assert_created()
        .get_body_object()
}

#[test]
fn get_all() {
    Client::new()
        .with_base(BASE)
        .get_all()
        .assert_ok()
        .get_body_array();
}

#[test]
fn get_one() {
    let mut client = Client::new();

    // setup
    let (user_id, user_token) = user::create(&mut client);
    let thread_id = thread::create(&mut client, &user_token);
    let created_value = create_event(&mut client, &user_token, thread_id);

    // test
    let body = client
        .with_base(BASE)
        .get(&created_value["id"])
        .assert_ok()
        .get_body_object();
    assert_eq!(created_value, body);

    // teardown
    client
        .with_base(BASE)
        .delete(Some(&user_token), &created_value["id"]);
    thread::delete(&mut client, &user_token, thread_id);
    user::delete(&mut client, user_id);
}

#[test]
fn create() {
    let mut client = Client::new();
    let (user_id, user_token) = user::create(&mut client);
    let thread_id = thread::create(&mut client, &user_token);

    let event = json!({
        "cols": [rand::random::<i64>(), guid!(), guid!()],
        "in_thread_id": thread_id,
    });

    let mut body = client
        .with_base(BASE)
        .post(Some(&user_token), &event)
        .assert_created()
        .get_body_object();
    assert!(body["id"].is_number(), r#"body["id"] is number"#);

    // store this so we can perform the teardown
    let id = body["id"].as_i64().unwrap();

    // Remove this, as we don't know what value we should expect.
    // Afterwards, we can ensure that the value is null.
    body["id"].take();
    assert_eq!(
        body,
        json!({
            "id": null,
            "posted": false,
            "cols": event["cols"],
            "in_thread_id": event["in_thread_id"],
        })
    );

    // teardown
    client.delete(Some(&user_token), id);
    thread::delete(&mut client, &user_token, thread_id);
    user::delete(&mut client, user_id);
}

#[test]
fn update() {
    let mut client = Client::new();

    // setup
    let (user_id, user_token) = user::create(&mut client);
    let thread_id = thread::create(&mut client, &user_token);
    let created_value = create_event(&mut client, &user_token, thread_id);
    assert_eq!(created_value["posted"].as_bool(), Some(false));

    // test
    // update a regular field
    let data = json!({ "posted": true });
    let body = client
        .with_base(BASE)
        .patch(Some(&user_token), &created_value["id"], &data)
        .assert_ok()
        .get_body_object();
    assert_eq!(body["posted"], data["posted"]);

    // update a specific column
    let data = json!([[2, guid!()]]);
    let body = client
        .with_base(BASE)
        .patch(Some(&user_token), &created_value["id"], &data)
        .assert_ok()
        .get_body_object();
    assert_eq!(body["cols"][2], data[0][1]);

    // teardown
    client
        .with_base(BASE)
        .delete(Some(&user_token), &created_value["id"]);
    thread::delete(&mut client, &user_token, thread_id);
    user::delete(&mut client, user_id);
}

#[test]
fn delete() {
    let mut client = Client::new();

    // setup
    let (user_id, user_token) = user::create(&mut client);
    let thread_id = thread::create(&mut client, &user_token);
    let created_value = create_event(&mut client, &user_token, thread_id);

    // test
    client
        .with_base(BASE)
        .delete(Some(&user_token), &created_value["id"])
        .assert_no_content();
    thread::delete(&mut client, &user_token, thread_id);
    user::delete(&mut client, user_id);
}
