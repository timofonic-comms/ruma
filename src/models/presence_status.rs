//! Storage and querying of presence status.

use diesel::{
    insert,
    Connection,
    ExecuteDsl,
    ExpressionMethods,
    FindDsl,
    FilterDsl,
    LoadDsl,
    SaveChangesDsl,
};
use diesel::expression::dsl::any;
use diesel::pg::PgConnection;
use diesel::pg::data_types::PgTimestamp;
use diesel::result::Error as DieselError;
use ruma_events::presence::PresenceState;
use ruma_identifiers::{UserId, EventId};
use time;

use error::ApiError;
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
    pub updated_at: PgTimestamp,
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

        // Use seconds instead of microseconds (default for PgTimestamp)
        self.updated_at = PgTimestamp(time::get_time().sec);

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

    /// Update based on current state.
    pub fn update_by_uid_and_status(
        connection: &PgConnection,
        homeserver_domain: &str,
        user_id: &UserId
    ) -> Result<(), ApiError> {
        let mut presence_state = PresenceState::Unavailable;
        let mut status_msg = None;

        match PresenceStatus::find_by_uid(connection, user_id)? {
            Some(status) => {
                presence_state = status.presence.parse()
                    .expect("Database insert should ensure a PresenceState");
                status_msg = status.status_msg;
            },
            None => (),
        }

        PresenceStatus::upsert(
            connection,
            homeserver_domain,
            user_id,
            presence_state,
            status_msg
        )
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

    /// Get status entries for a list of `UserId`'s which were updated after a
    /// specific point in time.
    pub fn get_users(
        connection: &PgConnection,
        users: &Vec<UserId>,
        since: Option<time::Timespec>,
    ) -> Result<Vec<PresenceStatus>, ApiError> {
        match since {
            Some(since) => {
                let time = PgTimestamp(since.sec);

                presence_status::table
                    .filter(presence_status::user_id.eq(any(users)))
                    .filter(presence_status::updated_at.gt(time))
                    .get_results(connection)
                    .map_err(ApiError::from)
            },
            None => {
                presence_status::table
                    .filter(presence_status::user_id.eq(any(users)))
                    .get_results(connection)
                    .map_err(ApiError::from)
            }
        }
    }
}
