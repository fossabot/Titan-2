use crate::{telemetry::ws_message, TELEMETRY};
use derive_more::{Constructor, Display};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Instant;

/// A request from a client to join certain rooms.
/// Each element should be able to be parsed with `Room::from_str`.
#[derive(Deserialize, Debug)]
pub struct JoinRequest {
    pub join: Vec<String>,
}

/// Room to send a `Message` to.
#[derive(Debug, Hash, PartialEq, Eq, Display, Clone, Copy)]
pub enum Room {
    #[display(fmt = "user")]
    User,
    #[display(fmt = "thread_create")]
    ThreadCreate,
    #[display(fmt = "thread_{}", _0)]
    Thread(i32),
}

impl std::str::FromStr for Room {
    type Err = &'static str;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        match string {
            "user" => Ok(Room::User),
            "thread_create" => Ok(Room::ThreadCreate),
            room if room.starts_with("thread_") => match room["thread_".len()..].parse() {
                Ok(id) => Ok(Room::Thread(id)),
                Err(_) => Err("invalid thread id"),
            },
            _ => Err("unknown room name"),
        }
    }
}

/// What action is the `data` field representing in a `Message`?
#[derive(Debug, Display)]
pub enum Action {
    #[display(fmt = "create")]
    Create,
    #[display(fmt = "update")]
    Update,
    #[display(fmt = "delete")]
    Delete,
}

/// What type is the `data` field in a `Message`?
#[derive(Debug, Display)]
pub enum DataType {
    #[display(fmt = "event")]
    Event,
    #[display(fmt = "section")]
    Section,
    #[display(fmt = "thread")]
    Thread,
    #[display(fmt = "user")]
    User,
}

/// A message that can be emitted to the various WebSocket clients.
/// Any serializable type can be sent as `data`,
/// though it should match the indicated `data_type`.
#[derive(Debug)]
pub struct Message<'a, T: Serialize> {
    pub room:      Room,
    pub action:    Action,
    pub data_type: DataType,
    pub data:      &'a T,
}

impl<T: Serialize> Message<'_, T> {
    pub fn send(&self) -> ws::Result<()> {
        let rooms = super::ROOMS.read();
        let clients = match rooms.get(&self.room) {
            Some(v) => v,
            None => return Ok(()),
        };

        let message = &*json!({
            "room": self.room.to_string(),
            "action": self.action.to_string(),
            "data_type": self.data_type.to_string(),
            "data": self.data,
        })
        .to_string();

        if *TELEMETRY {
            let send_start = Instant::now();
            for client in clients.iter() {
                let _ = client.send(message);
            }
            let elapsed = send_start.elapsed().as_micros();
            ws_message::log(message.len(), clients.len(), elapsed);
        } else {
            for client in clients.iter() {
                let _ = client.send(message);
            }
        }

        Ok(())
    }
}

/// Use this struct to add an `id` field to a preexisting struct.
/// The fields will be flattened by serde.
#[derive(Serialize, Constructor, Debug)]
pub struct Update<'a, T: Serialize> {
    pub id: i32,
    #[serde(flatten)]
    pub data: &'a T,
}
