#![feature(async_await, custom_attribute, decl_macro, proc_macro_hygiene)]
#![deny(rust_2018_idioms, clippy::all)]
#![warn(clippy::nursery)] // Don't deny, as there may be unknown bugs.
#![allow(intra_doc_link_resolution_failure)]

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
mod telemetry;
#[cfg(test)]
mod tests;
mod websocket;

use dotenv::dotenv;
use endpoint::{event, meta, oauth, section, thread, user};
use fairing::FeatureFilter;
use lazy_static::lazy_static;
use rocket::{
    config::{Config, Environment},
    routes,
    Rocket,
};
use rocket_conditional_attach::ConditionalAttach;
use rocket_contrib::{database, helmet::SpaceHelmet};
use rocket_cors::CorsOptions;
use rocket_telemetry::Telemetry;
use std::{error::Error, net::SocketAddr};

/// Single point to change if we need to alter the DBMS.
pub type Database = diesel::PgConnection;
#[database("data")]
pub struct DataDB(Database);

/// Returns a globally unique identifier.
/// Specifically, v4, which is not based on any input factors.
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
                    .default_value(
                        #[cfg(debug_assertions)]
                        "127.0.0.1:3000",
                        #[cfg(not(debug_assertions))]
                        "0.0.0.0:80",
                    )
                    .empty_values(false),
            )
            .arg(
                Arg::with_name("WebSocket host")
                    .help("Host IP & port for WebSocket connections")
                    .long("ws-host")
                    .value_name("IP_ADDR:PORT")
                    .default_value(
                        #[cfg(debug_assertions)]
                        "127.0.0.1:3001",
                        #[cfg(not(debug_assertions))]
                        "0.0.0.0:81",
                    )
                    .empty_values(false),
            )
            .arg(
                Arg::with_name("telemetry")
                    .help("Enables telemetry")
                    .short("t")
                    .long("telemetry"),
            )
            .get_matches()
    };
    static ref REST_HOST: SocketAddr =
        clap::value_t!(CLARGS.value_of("REST host"), SocketAddr).unwrap_or_else(|e| e.exit());
    static ref WS_HOST: SocketAddr =
        clap::value_t!(CLARGS.value_of("WebSocket host"), SocketAddr).unwrap_or_else(|e| e.exit());
    static ref TELEMETRY: bool = CLARGS.is_present("telemetry");
}

/// Creates a server,
/// attaching middleware for security and database access.
/// Routes are then mounted (some conditionally).
pub fn server() -> Rocket {
    let _ = dotenv();

    rocket::custom(
        Config::build(
            #[cfg(debug_assertions)]
            Environment::Development,
            #[cfg(not(debug_assertions))]
            Environment::Production,
        )
        .address(REST_HOST.ip().to_string())
        .port(REST_HOST.port())
        .unwrap(),
    )
    .attach(SpaceHelmet::default())
    .attach(CorsOptions::default().to_cors().unwrap())
    .attach(DataDB::fairing())
    .attach(FeatureFilter::default())
    .attach_if(*TELEMETRY, Telemetry::default())
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
fn main() -> Result<(), Box<dyn Error>> {
    use std::thread;

    thread::Builder::new()
        .name("websocket_server".into())
        .spawn(websocket::spawn)?;

    if *TELEMETRY {
        thread::Builder::new()
            .name("telemetry".into())
            .spawn(telemetry::spawn)?;
    }

    server().launch();

    Ok(())
}
