#![allow(non_snake_case)]

use super::{Event, Section, ToMarkdown, User, THREAD_CACHE_SIZE};
use crate::{
    schema::thread,
    websocket::{Action, DataType, Message, Room, Update},
    Database,
};
use enceladus_macros::generate_structs;
use lru_cache::LruCache;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use rocket_contrib::databases::diesel::{ExpressionMethods, QueryDsl, QueryResult, RunQueryDsl};
use serde::Deserialize;
use serde_json::{json, value::Value as Json};
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
static CACHE: Lazy<Mutex<LruCache<i32, Thread>>> =
    Lazy::new(|| Mutex::new(LruCache::new(THREAD_CACHE_SIZE)));

generate_structs! {
    Thread("thread") {
        auto id: i32,
        readonly thread_name: String,
        display_name: String,
        readonly post_id: Option<String>,
        readonly subreddit: Option<String>,
        space__t0: Option<i64>,
        video_url: Option<String>,
        spacex__api_id: Option<String>,
        readonly created_by_user_id: i32,
        sections_id: Vec<i32> = vec![],
        events_id: Vec<i32> = vec![],
        event_column_headers: Vec<String>,
        readonly space__utc_col_index: Option<i16>,
        is_live: bool = false,
    }
}

// Not all fields that are insertable should be provided by the user.
// Use an `ExternalInsertThread` wherever user input is expected.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalInsertThread {
    pub thread_name: String,
    pub display_name: String,
    pub subreddit: Option<String>,
    pub space__t0: Option<i64>,
    pub video_url: Option<String>,
    pub spacex__api_id: Option<String>,
    pub event_column_headers: Vec<String>,
    pub space__utc_col_index: Option<i16>,
    pub is_live: Option<bool>,
}

impl Thread {
    /// Find all `Thread`s in the database.
    ///
    /// Does _not_ use cache (reading or writing),
    /// so as to avoid storing values rarely accessed.
    pub fn find_all(conn: &Database) -> QueryResult<Vec<Self>> {
        use crate::schema::thread::dsl::thread;
        thread.load(conn)
    }

    /// Find a given `Thread` by its ID,
    /// joined with its `Section`s, `Event`s,
    /// each section's lock `User`, and the thread's created-by `User`.
    ///
    /// Note that this _does not_ query the database directly,
    /// so as to take advantage of cache wherever possible.
    /// Additionally, directly querying the database would make it more difficult
    /// to preserve the tree structure of the result.
    pub fn find_id_with_foreign_keys(conn: &Database, thread_id: i32) -> QueryResult<Json> {
        // Get the values, represented as normal structs.
        // For sections, we also add the relation to `User`,
        // so we represent those as raw, untyped JSON values.
        let raw_thread = Self::find_id(conn, thread_id)?;
        let created_by_user = User::find_id(conn, raw_thread.created_by_user_id)?;
        let sections: Vec<_> = raw_thread
            .sections_id
            .iter()
            .map(|section_id| Section::find_id(conn, *section_id).unwrap())
            .map(|section| {
                let user_id = section.lock_held_by_user_id;
                let mut section = serde_json::to_value(section).unwrap();
                section["lock_held_by_user"] = user_id.map_or(json!(null), |user_id| {
                    serde_json::to_value(User::find_id(conn, user_id).unwrap()).unwrap()
                });
                section
            })
            .collect();
        let events: Vec<_> = raw_thread
            .events_id
            .iter()
            .map(|event_id| Event::find_id(conn, *event_id).unwrap())
            .collect();

        // Convert the values to JSON,
        let mut thread_json = serde_json::to_value(raw_thread).unwrap();
        thread_json["created_by_user"] = serde_json::to_value(created_by_user).unwrap();
        thread_json["sections"] = serde_json::to_value(sections).unwrap();
        thread_json["events"] = serde_json::to_value(events).unwrap();

        Ok(thread_json)
    }

    /// Update a `Thread` on Reddit.
    ///
    /// This method will return `Ok(())` if the thread is not posted on Reddit.
    pub fn update_on_reddit(&self, conn: &Database) -> QueryResult<()> {
        if self.post_id.is_none() {
            return Ok(());
        }

        let mut user: reddit::User<'_> = User::find_id(conn, self.created_by_user_id)?.into();

        user.edit_self_post(
            &format!("t3_{}", self.post_id.clone().unwrap()),
            &self.to_markdown(conn).unwrap(),
        )
        .expect("error updating post on Reddit");

        User::update_access_token_if_necessary(conn, self.created_by_user_id, &mut user)
            .expect("could not update access token");

        Ok(())
    }

    /// Find a given `Thread` by its ID.
    ///
    /// Internally uses a cache to limit database accesses.
    pub fn find_id(conn: &Database, thread_id: i32) -> QueryResult<Self> {
        use crate::schema::thread::dsl::thread;

        let mut cache = CACHE.lock();
        if cache.contains_key(&thread_id) {
            Ok(cache.get_mut(&thread_id).unwrap().clone())
        } else {
            let result: Self = thread.find(thread_id).first(conn)?;
            cache.insert(thread_id, result.clone());
            Ok(result)
        }
    }

    /// Create a `Thread` given the data.
    ///
    /// The inserted row is added to the global cache and returned.
    pub fn create(
        conn: &Database,
        data: &ExternalInsertThread,
        user_id: i32,
        reddit_post_id: Option<String>,
    ) -> QueryResult<Self> {
        use crate::schema::thread::dsl::thread;

        let insertable_thread = InsertThread {
            thread_name: data.thread_name.clone(),
            display_name: data.display_name.clone(),
            post_id: reddit_post_id,
            subreddit: data.subreddit.clone(),
            space__t0: data.space__t0,
            video_url: data.video_url.clone(),
            spacex__api_id: data.spacex__api_id.clone(),
            created_by_user_id: user_id,
            events_id: vec![],
            sections_id: vec![],
            event_column_headers: data.event_column_headers.clone(),
            space__utc_col_index: data.space__utc_col_index,
            is_live: data.is_live.unwrap_or(false),
        };

        let result: Self = diesel::insert_into(thread)
            .values(insertable_thread)
            .get_result(conn)?;
        CACHE.lock().insert(result.id, result.clone());

        let _ = Message {
            room:      Room::ThreadCreate,
            action:    Action::Create,
            data_type: DataType::Thread,
            data:      &result,
        }
        .send();

        Ok(result)
    }

    /// Update a `Thread` given an ID and the data to update.
    ///
    /// The entry is updated in the database, added to cache, and returned.
    pub fn update(conn: &Database, thread_id: i32, data: &UpdateThread) -> QueryResult<Self> {
        use crate::schema::thread::dsl::{id, thread};

        let result: Self = diesel::update(thread)
            .filter(id.eq(thread_id))
            .set(data)
            .get_result(conn)?;
        CACHE.lock().insert(result.id, result.clone());

        let _ = Message {
            room:      Room::Thread(thread_id),
            action:    Action::Update,
            data_type: DataType::Thread,
            data:      &Update::new(thread_id, data),
        }
        .send();

        Ok(result)
    }

    /// Delete a `Thread` given its ID.
    ///
    /// Removes the entry from cache and returns the number of rows deleted (should be `1`).
    pub fn delete(conn: &Database, thread_id: i32) -> QueryResult<usize> {
        use crate::schema::thread::dsl::{id, thread};

        CACHE.lock().remove(&thread_id);

        let _ = Message {
            room:      Room::Thread(thread_id),
            action:    Action::Delete,
            data_type: DataType::Thread,
            data:      &json!({ "id": thread_id }),
        }
        .send();

        let removed_count = diesel::delete(thread)
            .filter(id.eq(thread_id))
            .execute(conn);

        if let Ok(removed_count) = removed_count {
            debug_assert_eq!(removed_count, 1);
        }

        removed_count
    }
}

impl ToMarkdown for Thread {
    /// Convert the `Thread` object to valid markdown.
    /// The resulting string is intended for consumption by Reddit,
    /// but should be valid for any markdown flavor supporting tables.
    fn to_markdown(&self, conn: &Database) -> Result<String, Box<dyn Error>> {
        let mut md = String::new();

        for &section_id in &self.sections_id {
            writeln!(
                &mut md,
                "{}\n",
                Section::find_id(conn, section_id)?.to_markdown(conn)?
            )?;
        }

        Ok(md)
    }
}
