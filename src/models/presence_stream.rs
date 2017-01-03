//! Storage of presence stream.

use std::time::SystemTime;

use diesel::{
    insert,
    ExecuteDsl,
};
use diesel::pg::PgConnection;
use ruma_identifiers::{EventId, UserId};

use error::ApiError;
use schema::presence_stream;

/// A Matrix presence stream, not saved yet.
#[derive(Debug, Clone, Insertable)]
#[table_name = "presence_stream"]
pub struct NewPresenceStreamEvent {
    /// The unique event ID.
    pub event_id: EventId,
    /// The user's ID.
    pub user_id: UserId,
    /// The current presence state.
    pub presence: String,
}

/// A Matrix presence stream.
#[derive(Debug, Queryable)]
pub struct PresenceStreamEvent {
    /// The depth of the event.
    pub ordering: i64,
    /// The unique event ID.
    pub event_id: EventId,
    /// The user's ID.
    pub user_id: UserId,
    /// The current presence state.
    pub presence: String,
    /// The time the event was created.
    pub created_at: SystemTime,
}


impl PresenceStreamEvent {
    /// Insert a `PresenceStreamEvent` entry.
    pub fn insert(connection: &PgConnection, event_id: &EventId, user_id: &UserId, presence: &String) -> Result<(), ApiError> {
        let new_event = NewPresenceStreamEvent {
            event_id: event_id.clone(),
            user_id: user_id.clone(),
            presence: presence.clone()
        };
        insert(&new_event)
            .into(presence_stream::table)
            .execute(connection)
            .map_err(ApiError::from)?;
        Ok(())
    }
}
