use rocket::get;
use serde_json::json;

/// Return information about the repository itself.
///
/// This endpoint is not versioned.
#[get("/")]
pub fn meta() -> String {
    json!({
        "version": env!("CARGO_PKG_VERSION"),
        "version_major": env!("CARGO_PKG_VERSION_MAJOR").parse::<u8>().unwrap(),
        "repository": env!("CARGO_PKG_REPOSITORY"),
    })
    .to_string()
}
