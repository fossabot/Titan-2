use super::{append_log, sleep, IncludesTimestamp};
use rocket_telemetry::Telemetry;

#[inline]
pub async fn log_requests() {
    loop {
        sleep(60).await;

        for request in Telemetry::reset().iter() {
            append_log(IncludesTimestamp(true), request.to_string());
        }
    }
}
