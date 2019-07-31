#![feature(async_await, custom_attribute, decl_macro, proc_macro_hygiene)]
#![deny(rust_2018_idioms, clippy::all, unsafe_code)]
#![warn(clippy::nursery)]

/// Needed for schema.rs - we can't inline it there, as it's auto-generated.
#[macro_use]
extern crate diesel;

mod controller;
mod encryption;
mod endpoint;
mod fairing;
mod schema;
mod telemetry;
#[cfg(test)]
mod tests;
mod websocket;

use dotenv::dotenv;
use endpoint::{event, meta, oauth, section, thread, user};
use fairing::FeatureFilter;
use once_cell::sync::Lazy;
use rocket::{routes, Rocket};
use rocket_conditional_attach::ConditionalAttach;
use rocket_contrib::{database, helmet::SpaceHelmet};
use rocket_cors::CorsOptions;
use rocket_telemetry::Telemetry;
use std::{error::Error, net::SocketAddr};

/// Single point to change if we need to alter the DBMS.
/// Note that there may be database-specific features that also need changing.
pub type Database = diesel::PgConnection;
#[database("data")]
pub struct DataDB(Database);

/// Returns a globally unique identifier.
/// Specifically, v4, which is not based on any input factors.
#[macro_export]
macro_rules! guid {
    () => {
        uuid::Uuid::new_v4().to_string()
    };
}

static CLARGS: Lazy<clap::ArgMatches<'_>> = Lazy::new(|| {
    use clap::{crate_authors, crate_description, crate_version, App, Arg};

    App::new("Enceladus API")
        .author(crate_authors!("\n"))
        .version(crate_version!())
        .about(crate_description!())
        .arg(
            Arg::with_name("REST host")
                .help("Host IP & port for HTTP requests")
                .short("r")
                .long("rest-host")
                .value_name("IP_ADDR:PORT")
                .default_value(
                    #[cfg(debug)]
                    "127.0.0.1:3000",
                    #[cfg(release)]
                    "0.0.0.0:80",
                )
                .empty_values(false),
        )
        .arg(
            Arg::with_name("WebSocket host")
                .help("Host IP & port for WebSocket connections")
                .short("w")
                .long("ws-host")
                .value_name("IP_ADDR:PORT")
                .default_value(
                    #[cfg(debug)]
                    "127.0.0.1:3001",
                    #[cfg(release)]
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
});

static REST_HOST: Lazy<SocketAddr> = Lazy::new(|| {
    clap::value_t!(CLARGS.value_of("REST host"), SocketAddr).unwrap_or_else(|e| e.exit())
});
static WS_HOST: Lazy<SocketAddr> = Lazy::new(|| {
    clap::value_t!(CLARGS.value_of("WebSocket host"), SocketAddr).unwrap_or_else(|e| e.exit())
});
static TELEMETRY: Lazy<bool> = Lazy::new(|| CLARGS.is_present("telemetry"));

/// Creates a server,
/// attaching middleware for security and database access.
/// Routes are then mounted (some conditionally).
pub fn server() -> Rocket {
    let _ = dotenv();

    #[cfg(debug)]
    std::env::set_var("ROCKET_ENV", "development");
    #[cfg(release)]
    std::env::set_var("ROCKET_ENV", "production");
    std::env::set_var("ROCKET_HOST", REST_HOST.ip().to_string());
    std::env::set_var("ROCKET_PORT", REST_HOST.port().to_string());

    rocket::ignite()
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
            #[cfg(debug)]
            routes![user::all, user::get, user::post, user::patch, user::delete],
            #[cfg(release)]
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
