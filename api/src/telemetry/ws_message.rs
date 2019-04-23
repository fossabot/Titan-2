use super::{append_log, IncludesTimestamp};

#[inline]
pub fn log_sent_message(message_length: usize, clients: usize, microseconds: u128) {
    append_log(
        IncludesTimestamp(false),
        format!(
            "Sent WebSocket message ({bytes} bytes) to {clients} clients in {µs}µs",
            bytes = message_length,
            clients = clients,
            µs = microseconds,
        ),
    );
}
