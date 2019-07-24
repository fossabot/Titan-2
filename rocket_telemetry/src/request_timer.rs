use derive_deref::Deref;
use std::time::SystemTime;

#[derive(Debug, Deref)]
pub struct RequestTimer(Option<SystemTime>);

impl RequestTimer {
    pub fn begin() -> Self {
        Self(Some(SystemTime::now()))
    }

    pub const fn end() -> Self {
        Self(None)
    }
}
