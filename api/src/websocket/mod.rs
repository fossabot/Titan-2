use crate::WS_HOST;
use hashbrown::{HashMap, HashSet};
use lazy_static::lazy_static;
use parking_lot::RwLock;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use ws::{CloseCode, Handler, Handshake, Message as WsMessage, Sender};

mod structs;
pub use structs::*;

lazy_static! {
    // We're using `Arc` and not `Weak`,
    // as the latter doesn't implement `Hash`.
    // As such, we have to manually drop the reference
    // in the `on_close` method to prevent a memory leak.
    static ref ROOMS: RwLock<HashMap<Room, HashSet<Arc<Sender>>>> = RwLock::new(HashMap::new());
}

pub static CONNECTED_CLIENTS: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug)]
struct Socket {
    out:   Arc<Sender>,
    rooms: HashSet<Room>,
}

impl Handler for Socket {
    #[inline(always)]
    fn on_open(&mut self, _: Handshake) -> ws::Result<()> {
        CONNECTED_CLIENTS.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    #[inline]
    fn on_message(&mut self, message: WsMessage) -> ws::Result<()> {
        let message = match message {
            WsMessage::Text(s) => s,
            _ => return Ok(()),
        };

        let mut rooms = ROOMS.write();

        for room in match serde_json::from_str(&message) {
            Ok(JoinRequest { join }) => join,
            _ => return Ok(()),
        }
        .into_iter()
        .filter_map(|s| s.parse().ok())
        {
            // Store the connection itself in the global room.
            rooms
                .entry(room)
                .or_insert(HashSet::new())
                .insert(Arc::clone(&self.out));

            // Store the connection's rooms on the instance.
            self.rooms.insert(room);
        }

        Ok(())
    }

    #[inline]
    fn on_close(&mut self, _code: CloseCode, _reason: &str) {
        // Avoid locking the map if we don't need to.
        if !self.rooms.is_empty() {
            let mut rooms = ROOMS.write();

            // Leave all rooms the user is currently in.
            // These should be the final references to the values,
            // so doing this should call `Drop` and free up the memory.
            for room in self.rooms.iter() {
                rooms.get_mut(room).unwrap().remove(&self.out);
            }
        }

        CONNECTED_CLIENTS.fetch_sub(1, Ordering::Relaxed);
    }
}

#[inline]
pub fn spawn() {
    ws::listen(WS_HOST.to_string(), |out| Socket {
        out:   Arc::new(out),
        rooms: HashSet::new(),
    })
    .unwrap();
}
