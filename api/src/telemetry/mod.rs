mod requests;
mod ws_clients;
pub mod ws_message;

use chrono::prelude::*;
use derive_deref::Deref;
use futures::{
    compat::Future01CompatExt,
    future::{FutureExt, TryFutureExt},
};
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use std::time::{Duration, Instant};
use tokio::{fs::file::File, prelude::*, timer::Delay};

#[derive(Clone, Copy, Debug, Deref)]
struct IncludesTimestamp(bool);

const LOG_FILE_NAME: &str = "logs.txt";
static LOG_FILE: Lazy<RwLock<File>> = Lazy::new(|| {
    RwLock::new(
        std::fs::OpenOptions::new()
            .read(true)
            .append(true)
            .create(true)
            .open(LOG_FILE_NAME)
            .map(File::from_std)
            .expect("Could not open log file"),
    )
});

/// Resolve the future after the provided number of seconds.
async fn sleep(seconds: u64) {
    Delay::new(Instant::now() + Duration::from_secs(seconds))
        .compat()
        .await
        .expect("Error in tokio timer");
}

fn append_log(includes_timestamp: IncludesTimestamp, message: impl Into<Vec<u8>>) {
    // Prevent reallocating as long as the message isn't terribly long.
    let mut bytes = Vec::with_capacity(512);

    // Prepend a timestamp if one is not provided.
    if !*includes_timestamp {
        // Current time in UTC.
        bytes.append(&mut Utc::now().format("%Y%m%dT%H%M%SZ ").to_string().into());
    }

    // The message provided by the caller.
    bytes.append(&mut message.into());

    // A newline for sanity.
    bytes.push(b'\n');

    // Write to the log file using tokio's `AsyncWrite` trait.
    LOG_FILE
        .write()
        .poll_write(&bytes)
        .expect("Error writing to file");
}

pub fn spawn() {
    macro_rules! compat {
        ($x:expr) => {
            $x.unit_error().boxed().compat()
        };
    }

    tokio::run(compat!(async {
        tokio::spawn(compat!(requests::log()));
        tokio::spawn(compat!(ws_clients::log()));
    }));
}
