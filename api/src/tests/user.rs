use crate::{guid, tests::helpers::*};
use serde_json::{json, Value as Json};

const BASE: &str = "/v1/user";

fn create_user(client: &mut Client<'_>) -> Json {
    client
        .with_base(BASE)
        .post(
            None,
            json!({
                "reddit_username": guid!(),
                "refresh_token": guid!(),
                "access_token": guid!(),
                "access_token_expires_at_utc": 0,
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
    let created_value = create_user(&mut client);

    // test
    let body = client
        .with_base(BASE)
        .get(&created_value["id"])
        .assert_ok()
        .get_body_object();

    // The token field only exists in testing,
    // and is only returned on the "create" endpoint.
    assert_eq!(
        body,
        json!({
            "id": created_value["id"],
            "reddit_username": created_value["reddit_username"],
            "lang": created_value["lang"],
            "is_global_admin": created_value["is_global_admin"],
        })
    );

    // teardown
    user::delete(&mut client, created_value["id"].as_i64().unwrap() as i32);
}

#[test]
fn create() {
    let mut client = Client::new();

    let user = json!({
        "reddit_username": guid!(),
        "refresh_token": guid!(),
        "access_token": guid!(),
        "access_token_expires_at_utc": 0,
    });

    let mut body = client
        .with_base(BASE)
        .post(None, &user)
        .assert_created()
        .get_body_object();
    assert!(body["id"].is_number(), r#"body["id"] is number"#);
    assert_eq!(body.get("refresh_token"), None);

    // store this so we can perform the teardown
    let id = body["id"].as_i64().unwrap() as i32;

    // Remove this, as we don't know what value we should expect.
    // Afterwards, we can ensure that the value is null.
    body["id"].take();
    body["token"].take();
    assert_eq!(
        body,
        json!({
            "token": null,
            "id": null,
            "reddit_username": user["reddit_username"],
            "lang": "en",
            "is_global_admin": false,
        })
    );

    // teardown
    user::delete(&mut client, id);
}

#[test]
fn update() {
    let mut client = Client::new();

    // setup
    let created_value = create_user(&mut client);
    assert_eq!(created_value["is_global_admin"].as_bool(), Some(false));

    // test
    let data = json!({ "is_global_admin": true });
    let body = client
        .with_base(BASE)
        .patch(None, &created_value["id"], &data)
        .assert_ok()
        .get_body_object();

    assert_eq!(body["is_global_admin"], data["is_global_admin"],);

    // teardown
    user::delete(&mut client, created_value["id"].as_i64().unwrap() as i32)
}

#[test]
fn delete() {
    let mut client = Client::new();

    // setup
    let created_value = create_user(&mut client);

    // test
    client
        .with_base(BASE)
        .delete(None, &created_value["id"])
        .assert_no_content();
}
