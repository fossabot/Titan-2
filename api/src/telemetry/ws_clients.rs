use super::{append_log, sleep, IncludesTimestamp};
use crate::websocket::CONNECTED_CLIENTS;
use std::sync::atomic::Ordering;

#[inline]
pub async fn log_ws_clients() {
    loop {
        sleep(10).await;

        append_log(
            IncludesTimestamp(false),
            format!(
                "WebSocket connections: {}",
                CONNECTED_CLIENTS.load(Ordering::Relaxed),
            ),
        );
    }
}
