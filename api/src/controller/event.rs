use super::{Thread, ToMarkdown, UpdateThread, EVENT_CACHE_SIZE};
use crate::{
    schema::event,
    websocket::{Action, DataType, Message, Room, Update},
    Database,
};
use lru_cache::LruCache;
use macros::generate_structs;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use rocket_contrib::databases::diesel::{ExpressionMethods, QueryDsl, QueryResult, RunQueryDsl};
use serde_json::json;
use std::{error::Error, fmt::Write};

/// A global cache, containing a mapping of IDs to their respective `Event`.
///
/// The cache is protected by a `Mutex`,
/// ensuring there is only ever at most one writer at a time.
/// Note that even when reading,
/// there must be a lock on mutability,
/// as the `LruCache` must be able to update itself.
///
/// To read from the cache,
/// you'll want to call `CACHE.lock()` before performing normal operations.
/// ```
static CACHE: Lazy<Mutex<LruCache<i32, Event>>> =
    Lazy::new(|| Mutex::new(LruCache::new(EVENT_CACHE_SIZE)));

generate_structs! {
    Event("event") {
        auto id: i32,
        posted: bool = false,
        readonly in_thread_id: i32,
        cols: serde_json::Value,
    }
}

impl Event {
    /// Find all `Event`s in the database.
    ///
    /// Does _not_ use cache (reading or writing),
    /// so as to avoid storing values rarely accessed.
    pub fn find_all(conn: &Database) -> QueryResult<Vec<Self>> {
        use crate::schema::event::dsl::event;
        event.load(conn)
    }

    /// Find a given `Event` by its ID.
    ///
    /// Internally uses a cache to limit database accesses.
    pub fn find_id(conn: &Database, event_id: i32) -> QueryResult<Self> {
        use crate::schema::event::dsl::event;

        let mut cache = CACHE.lock();
        if cache.contains_key(&event_id) {
            Ok(cache.get_mut(&event_id).unwrap().clone())
        } else {
            let result: Self = event.find(event_id).first(conn)?;
            cache.insert(event_id, result.clone());
            Ok(result)
        }
    }

    /// Create an `Event` given the data.
    ///
    /// The inserted row is added to the global cache and returned.
    pub fn create(conn: &Database, data: &InsertEvent) -> QueryResult<Self> {
        use crate::schema::event::dsl::event;

        let result: Self = diesel::insert_into(event).values(data).get_result(conn)?;
        CACHE.lock().insert(result.id, result.clone());

        let _ = Message {
            room:      Room::Thread(result.in_thread_id),
            action:    Action::Create,
            data_type: DataType::Event,
            data:      &result,
        }
        .send();

        // Add the event ID to the relevant Thread.
        let mut thread = Thread::find_id(conn, data.in_thread_id)?;
        thread.events_id.push(result.id);
        Thread::update(
            conn,
            data.in_thread_id,
            &UpdateThread {
                events_id: thread.events_id.into(),
                ..UpdateThread::default()
            },
        )?;

        Ok(result)
    }

    /// Update an `Event` given an ID and the data to update.
    ///
    /// The entry is updated in the database, added to cache, and returned.
    pub fn update(conn: &Database, event_id: i32, data: &UpdateEvent) -> QueryResult<Self> {
        use crate::schema::event::dsl::{event, id};

        let result: Self = diesel::update(event)
            .filter(id.eq(event_id))
            .set(data)
            .get_result(conn)?;
        CACHE.lock().insert(result.id, result.clone());

        let _ = Message {
            room:      Room::Thread(result.in_thread_id),
            action:    Action::Update,
            data_type: DataType::Event,
            data:      &Update::new(event_id, data),
        }
        .send();

        Ok(result)
    }

    /// Delete an `Event` given its ID.
    ///
    /// Removes the entry from cache and returns the number of rows deleted (should be `1`).
    pub fn delete(conn: &Database, event_id: i32) -> QueryResult<usize> {
        use crate::schema::event::dsl::{event, id};

        let mut thread = Thread::find_id(conn, Self::find_id(conn, event_id)?.in_thread_id)?;
        thread.events_id.retain(|&cur_id| cur_id != event_id);
        Thread::update(
            conn,
            thread.id,
            &UpdateThread {
                events_id: thread.events_id.into(),
                ..UpdateThread::default()
            },
        )?;

        let _ = Message {
            room:      Room::Thread(thread.id),
            action:    Action::Delete,
            data_type: DataType::Event,
            data:      &json!({ "id": event_id }),
        }
        .send();

        CACHE.lock().remove(&event_id);

        let removed_count = diesel::delete(event).filter(id.eq(event_id)).execute(conn);

        if let Ok(removed_count) = removed_count {
            debug_assert_eq!(removed_count, 1);
        }

        removed_count
    }
}

impl ToMarkdown for Event {
    /// Convert the `Event` object to valid markdown.
    /// The resulting string is intended for consumption by Reddit,
    /// but should be valid for any markdown flavor supporting tables.
    ///
    /// The designated UTC column (if any) will be formatted as `HH:MM`.
    fn to_markdown(&self, conn: &Database) -> Result<String, Box<dyn Error>> {
        if !self.posted {
            return Ok("".into());
        }

        let mut md = String::new();

        if !self.cols.is_array() {
            panic!("Expected columns to be array");
        }

        let utc_col_index = Thread::find_id(conn, self.in_thread_id)?.space__utc_col_index;

        for (val, i) in self.cols.as_array().unwrap().iter().zip(0..) {
            write!(
                &mut md,
                "|{}",
                // If the column in question is the designated UTC timestamp,
                // format it as such.
                if utc_col_index == Some(i) {
                    let timestamp = val.as_i64().expect("expected i64 in UTC column");
                    let hours = timestamp % 86_400 / 3_600;
                    let minutes = timestamp % 3_600 / 60;

                    format!("{:02}:{:02}", hours, minutes)
                } else {
                    use serde_json::Value::{Number, String};
                    match val {
                        Number(n) => n.clone().as_i64().unwrap().to_string(),
                        String(s) => s.clone().to_owned(),
                        _ => panic!("expected number or string"),
                    }
                }
                .replace('\n', " ")
                .replace('|', "\\|")
            )?;
        }

        writeln!(&mut md, "|")?;

        Ok(md)
    }
}
