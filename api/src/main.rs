#![feature(
    async_await,
    custom_attribute,
    decl_macro,
    proc_macro_hygiene
)]
#![deny(rust_2018_idioms, clippy::all)]
#![warn(clippy::nursery)] // Don't deny, as there may be unknown bugs.
#![allow(intra_doc_link_resolution_failure, clippy::match_bool)]

#[macro_use]
extern crate diesel;

#[macro_use]
extern crate dotenv_codegen;

mod controller;
mod encryption;
mod endpoint;
mod fairing;
mod rocket_conditional_attach;
mod schema;
#[cfg(feature = "telemetry")]
mod telemetry;
mod websocket;

#[cfg(test)]
mod tests;

use dotenv::dotenv;
use endpoint::*;
use fairing::*;
use rocket::{routes, Rocket};
use rocket_conditional_attach::*;
use rocket_contrib::{database, helmet::SpaceHelmet};
use rocket_cors::Cors;
#[cfg(feature = "telemetry")]
use rocket_telemetry::Telemetry;

/// Single point to change if we need to alter the DBMS.
pub type Database = diesel::PgConnection;
#[database("data")]
pub struct DataDB(Database);

/// Returns a globally unique identifier.
/// Specifically, v4, which is not based on any input factors.
#[inline]
pub fn guid() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Creates a server,
/// attaching middleware for security and database access.
/// Routes are then mounted (some conditionally).
#[inline]
pub fn server() -> Rocket {
    // Although we inline most variables at compile-time,
    // Rocket doesn't let us do this.
    // As such, we have to load the `.env` file at runtime as well.
    // **Do not** remove it from the build script,
    // as we still want to inline wherever possible.
    dotenv().ok();

    rocket::ignite()
        .attach(SpaceHelmet::default())
        .attach(Cors::default())
        .attach(DataDB::fairing())
        .attach(FeatureFilter::default())
        .attach_if(cfg!(feature = "telemetry"), Telemetry::default())
        .manage(Cors::default())
        .mount("/", rocket_cors::catch_all_options_routes())
        .mount("/meta", routes![meta::meta])
        .mount("/oauth", routes![oauth::oauth, oauth::callback])
        .mount(
            "/v1/user",
            #[cfg(debug_assertions)]
            routes![user::all, user::get, user::post, user::patch, user::delete],
            #[cfg(not(debug_assertions))]
            routes![user::all, user::get],
        )
        .mount(
            "/v1/thread",
            routes![
                thread::all,
                thread::get,
                thread::get_full,
                thread::post,
                thread::patch,
                thread::approve,
                thread::sticky,
                thread::unsticky,
                thread::delete,
            ],
        )
        .mount(
            "/v1/section",
            routes![
                section::all,
                section::get,
                section::post,
                section::patch,
                section::delete,
            ],
        )
        .mount(
            "/v1/event",
            routes![
                event::all,
                event::get,
                event::post,
                event::patch,
                event::delete,
            ],
        )
}

/// Launch the server.
/// Uses the port number defined in the environment variable `ROCKET_PORT`.
/// If not defined, defaults to `8000`.
fn main() {
    std::thread::Builder::new()
        .name("websocket_server".into())
        .spawn(|| {
            websocket::spawn();
        })
        .unwrap();

    #[cfg(feature = "telemetry")]
    std::thread::Builder::new()
        .name("telemetry".into())
        .spawn(|| {
            telemetry::spawn();
        })
        .unwrap();

    server().launch();
}
