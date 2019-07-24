use super::{append_log, sleep, IncludesTimestamp};
use rocket_telemetry::Telemetry;

pub async fn log() {
    loop {
        sleep(60).await;

        for request in Telemetry::reset() {
            append_log(IncludesTimestamp(true), request.to_string());
        }
    }
}
