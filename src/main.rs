#![feature(proc_macro_hygiene, decl_macro, concat_idents, custom_attribute)]
#![deny(warnings, clippy::all)]
#![allow(
    proc_macro_derive_resolution_fallback,
    unused_attributes,
    intra_doc_link_resolution_failure,
    clippy::match_bool
)]

#[macro_use]
extern crate diesel;

mod controller;
mod endpoint;
mod schema;

#[cfg(test)]
mod tests;

use crate::endpoint::*;
use dotenv::dotenv;
use rocket::{routes, Rocket};
use rocket_contrib::{database, helmet::SpaceHelmet};

// single point to change if we need to alter the DBMS
pub type Database = diesel::PgConnection;
#[database("data")]
pub struct DataDB(Database);

macro_rules! all_routes {
    ($ns:ident) => {
        routes![$ns::all, $ns::get, $ns::post, $ns::patch, $ns::delete]
    };
}

#[inline]
pub fn guid() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Creates a server,
/// attaching middleware for security and database access.
/// Routes are then mounted (some conditionally).
pub fn server() -> Rocket {
    dotenv().ok();

    rocket::ignite()
        .attach(SpaceHelmet::default())
        .attach(DataDB::fairing())
        .mount("/meta", routes![meta::meta])
        .mount("/oauth", routes![oauth::oauth, oauth::callback])
        .mount(
            "/v1/user",
            match cfg!(debug_assertions) {
                true => all_routes!(user),
                false => routes![user::all, user::get],
            },
        )
        .mount("/v1/preset_event", all_routes!(preset_event))
        .mount("/v1/thread", all_routes!(thread))
        .mount("/v1/section", all_routes!(section))
        .mount("/v1/event", all_routes!(event))
}

/// Launch the server.
fn main() {
    server().launch();
}
