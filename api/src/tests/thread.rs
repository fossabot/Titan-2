use crate::{guid, tests::helpers::*};
use serde_json::{json, Value as Json};

const BASE: &str = "/v1/thread";

fn create_thread(client: &mut Client<'_>, token: &str) -> Json {
    client
        .with_base(BASE)
        .post(
            Some(token),
            json!({
                "thread_name": guid!(),
                "display_name": guid!(),
                "event_column_headers": [],
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
    let created_value = create_thread(&mut client, &user_token);

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
        .delete(Some(&user_token), &body["id"]);
    user::delete(&mut client, user_id);
}

#[test]
fn create() {
    let mut client = Client::new();
    let (user_id, user_token) = user::create(&mut client);

    let thread = json!({
        "thread_name": guid!(),
        "display_name": guid!(),
        "space__t0": rand::random::<i64>(),
        "video_url": guid!(),
        "event_column_headers": [],
    });

    let mut body = client
        .with_base(BASE)
        .post(Some(&user_token), &thread)
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
            // auto-generated
            "id": null,
            "created_by_user_id": user_id,
            "post_id": null,
            "subreddit": null,
            "sections_id": [],
            "events_id": [],
            "is_live": false,

            // user-provided
            "thread_name": thread["thread_name"],
            "display_name": thread["display_name"],
            "video_url": thread["video_url"],
            "event_column_headers": thread["event_column_headers"],
        })
    );

    // teardown
    client.with_base(BASE).delete(Some(&user_token), id);
    user::delete(&mut client, user_id);
}

#[test]
#[should_panic]
fn create_no_auth() {
    let mut client = Client::new();
    let thread = json!({
        "thread_name": guid!(),
        "display_name": guid!(),
        "event_column_headers": [],
    });

    client.with_base(BASE).post(None, &thread).assert_created();
}

#[test]
fn update() {
    let mut client = Client::new();

    // setup
    let (user_id, user_token) = user::create(&mut client);
    let created_value = create_thread(&mut client, &user_token);
    assert_eq!(created_value["video_url"].as_str(), None);

    // test
    let data = json!({ "video_url": guid!() });
    let body = client
        .with_base(BASE)
        .patch(Some(&user_token), &created_value["id"], &data)
        .assert_ok()
        .get_body_object();
    assert_eq!(body["video_url"], data["video_url"]);

    // teardown
    client
        .with_base(BASE)
        .delete(Some(&user_token), &created_value["id"]);
    user::delete(&mut client, user_id);
}

#[test]
fn delete() {
    let mut client = Client::new();

    // setup
    let (user_id, user_token) = user::create(&mut client);
    let created_value = create_thread(&mut client, &user_token);

    // test
    client
        .with_base(BASE)
        .delete(Some(&user_token), &created_value["id"])
        .assert_no_content();
    user::delete(&mut client, user_id);
}
