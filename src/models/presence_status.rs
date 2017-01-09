//! Storage and querying of presence status.

#[cfg(test)]
use std::time::Duration;
use std::time::SystemTime;

use diesel::{
    insert,
    Connection,
    ExecuteDsl,
    FindDsl,
    LoadDsl,
    SaveChangesDsl,
};
use diesel::result::Error as DieselError;
use diesel::pg::PgConnection;
use ruma_events::presence::PresenceState;
use ruma_identifiers::{UserId, EventId};

use error::ApiError;
use models::presence_event::PresenceStreamEvent;
use schema::presence_status;

/// A Matrix presence status, not saved yet.
#[derive(Debug, Clone, Insertable)]
#[table_name = "presence_status"]
pub struct NewPresenceStatus {
    /// The user's ID.
    pub user_id: UserId,
    /// The unique event ID.
    pub event_id: EventId,
    /// The current presence state.
    pub presence: String,
    /// A possible status message from the user.
    pub status_msg: Option<String>
}

/// A Matrix presence status.
#[derive(Debug, Clone, Queryable, Identifiable, AsChangeset)]
#[table_name = "presence_status"]
#[primary_key(user_id)]
pub struct PresenceStatus {
    /// The user's ID.
    pub user_id: UserId,
    /// The unique event ID.
    pub event_id: EventId,
    /// The current presence state.
    pub presence: String,
    /// A possible status message from the user.
    pub status_msg: Option<String>,
    /// Timestamp of the last update.
    pub updated_at: SystemTime,
}

impl PresenceStatus {
    /// Update or insert a presence status entry.
    pub fn upsert(
        connection: &PgConnection,
        homeserver_domain: &str,
        user_id: &UserId,
        presence: PresenceState,
        status_msg: Option<String>
    ) -> Result<(), ApiError> {
        let event_id = &EventId::new(&homeserver_domain).map_err(ApiError::from)?;
        connection.transaction::<(), ApiError, _>(|| {
            let status = PresenceStatus::find_by_uid(connection, user_id)?;
            PresenceStreamEvent::insert(connection, event_id, user_id, presence)?;
            match status {
                Some(mut status) => status.update(connection, presence, status_msg, event_id),
                None => PresenceStatus::create(connection, user_id, presence, status_msg, event_id),
            }
        }).map_err(ApiError::from)
    }

    /// Update a presence status entry.
    fn update(
        &mut self,
        connection: &PgConnection,
        presence: PresenceState,
        status_msg: Option<String>,
        event_id: &EventId
    ) -> Result<(), ApiError> {
        self.presence = presence.to_string();
        self.status_msg = status_msg;
        self.event_id = event_id.clone();
        self.updated_at = SystemTime::now();

        match self.save_changes::<PresenceStatus>(connection) {
            Ok(_) => Ok(()),
            Err(error) => Err(ApiError::from(error)),
        }
    }

    /// Create a presence status entry.
    fn create(
        connection: &PgConnection,
        user_id: &UserId,
        presence: PresenceState,
        status_msg: Option<String>,
        event_id: &EventId
    ) -> Result<(), ApiError> {
        let new_status = NewPresenceStatus {
            user_id: user_id.clone(),
            event_id: event_id.clone(),
            presence: presence.to_string(),
            status_msg: status_msg,
        };
        insert(&new_status)
            .into(presence_status::table)
            .execute(connection)
            .map_err(ApiError::from)?;
        Ok(())
    }

    /// Return `PresenceStatus` for given `UserId`.
    pub fn find_by_uid(
        connection: &PgConnection,
        user_id: &UserId
    ) -> Result<Option<PresenceStatus>, ApiError> {
        let status = presence_status::table.find(user_id).first(connection);

        match status{
            Ok(status) => Ok(Some(status)),
            Err(DieselError::NotFound) => Ok(None),
            Err(err) => Err(ApiError::from(err)),
        }
    }

    /// Calculate the difference between two SystemTimes in milliseconds.
    pub fn calculate_time_difference(since: SystemTime, now: SystemTime) -> Result<u64, ApiError> {
        let elapsed = now.duration_since(since).map_err(ApiError::from)?;
        let mut millis = elapsed.as_secs() * 1_000;
        millis += (elapsed.subsec_nanos() / 1_000_000) as u64;
        Ok(millis)
    }
}

#[test]
fn calculate_last_active_ago_work_correctly() {
    let now = SystemTime::now();
    assert_eq!(PresenceStatus::calculate_time_difference(now, now).unwrap(), 0);
    let now = SystemTime::now();
    let added = now + Duration::from_millis(1500);
    assert_eq!(PresenceStatus::calculate_time_difference(now, added).unwrap(), 1500);
}
