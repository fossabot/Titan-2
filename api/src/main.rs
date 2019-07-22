#![feature(async_await, custom_attribute, decl_macro, proc_macro_hygiene)]
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
use lazy_static::lazy_static;
use rocket::{
    config::{Config, Environment},
    routes,
    Rocket,
};
use rocket_conditional_attach::*;
use rocket_contrib::{database, helmet::SpaceHelmet};
use rocket_cors::CorsOptions;
#[cfg(feature = "telemetry")]
use rocket_telemetry::Telemetry;
use std::net::SocketAddr;

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

lazy_static! {
    static ref CLARGS: clap::ArgMatches<'static> = {
        use clap::{crate_authors, crate_description, crate_version, App, Arg};

        App::new("Enceladus API")
            .author(crate_authors!("\n"))
            .version(crate_version!())
            .about(crate_description!())
            .arg(
                Arg::with_name("REST host")
                    .help("Host IP & port for HTTP requests")
                    .long("rest-host")
                    .value_name("IP_ADDR:PORT")
                    .default_value(match cfg!(debug_assertions) {
                        true => "127.0.0.1:3000",
                        false => "0.0.0.0:80",
                    })
                    .empty_values(false),
            )
            .arg(
                Arg::with_name("WebSocket host")
                    .help("Host IP & port for WebSocket connections")
                    .long("ws-host")
                    .value_name("IP_ADDR:PORT")
                    .default_value(match cfg!(debug_assertions) {
                        true => "127.0.0.1:3001",
                        false => "0.0.0.0:81",
                    })
                    .empty_values(false),
            )
            .get_matches()
    };
    static ref REST_HOST: SocketAddr =
        clap::value_t!(CLARGS.value_of("rest_host"), SocketAddr).unwrap_or_else(|e| e.exit());
    static ref WS_HOST: SocketAddr =
        clap::value_t!(CLARGS.value_of("ws_host"), SocketAddr).unwrap_or_else(|e| e.exit());
}

/// Creates a server,
/// attaching middleware for security and database access.
/// Routes are then mounted (some conditionally).
#[inline]
pub fn server() -> Rocket {
    dotenv().ok();

    rocket::custom(
        Config::build(match cfg!(debug_assertions) {
            true => Environment::Development,
            false => Environment::Production,
        })
        .address(REST_HOST.ip().to_string())
        .port(REST_HOST.port())
        .unwrap(),
    )
    .attach(SpaceHelmet::default())
    .attach(CorsOptions::default().to_cors().unwrap())
    .attach(DataDB::fairing())
    .attach(FeatureFilter::default())
    .attach_if(cfg!(feature = "telemetry"), Telemetry::default())
    .manage(CorsOptions::default().to_cors().unwrap())
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
