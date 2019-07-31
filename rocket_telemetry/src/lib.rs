#![feature(duration_float)] // https://github.com/rust-lang/rust/pull/62756
#![deny(rust_2018_idioms, clippy::all, unsafe_code)]
#![warn(clippy::nursery)]

mod request_log;
mod request_timer;

use once_cell::sync::Lazy;
use parking_lot::RwLock;
use request_timer::Timer;
use rocket::{
    fairing::{Fairing, Info, Kind},
    http::{Method, Status},
    Data,
    Request,
    Response,
};
use std::{io::Cursor, mem};

pub(crate) type RequestLog = Vec<request_log::Entry>;
pub(crate) static REQUESTS: Lazy<RwLock<RequestLog>> = Lazy::new(|| RwLock::new(Vec::new()));

#[derive(Debug, Default)]
pub struct Telemetry;

impl Telemetry {
    /// Reset the telemetry to a fresh state,
    /// returning the existing logs.
    pub fn reset() -> RequestLog {
        mem::replace(&mut REQUESTS.write(), vec![])
    }
}

impl Fairing for Telemetry {
    fn info(&self) -> Info {
        Info {
            name: "Telemetry",
            kind: Kind::Request | Kind::Response,
        }
    }

    fn on_request(&self, request: &mut Request<'_>, _: &Data) {
        request.local_cache(Timer::begin);
    }

    fn on_response(&self, request: &Request<'_>, response: &mut Response<'_>) {
        let start_time = request
            .local_cache(Timer::end)
            .expect("unable to get request start time");
        let duration = start_time.elapsed().expect("error with system time");

        let status = response.status();
        let method = request.method();
        if status == Status::NotFound || method == Method::Options {
            return;
        }

        let body_size = match response.body_bytes() {
            Some(body) => {
                let len = body.len();
                response.set_sized_body(Cursor::new(body));
                len
            }
            None => 0,
        };

        REQUESTS.write().push(request_log::Entry {
            method,
            uri: request.uri().path().to_string(),
            status,
            body_size,
            duration,
            start_time,
        });
    }
}
