use derive_deref::Deref;
use std::time::SystemTime;

#[derive(Debug, Deref)]
pub struct Timer(Option<SystemTime>);

impl Timer {
    pub fn begin() -> Self {
        Self(Some(SystemTime::now()))
    }

    pub const fn end() -> Self {
        Self(None)
    }
}
