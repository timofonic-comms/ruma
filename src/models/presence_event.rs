//! Storage of presence stream.

use std::time::SystemTime;

use diesel::{
    insert,
    ExpressionMethods,
    FilterDsl,
    GroupByDsl,
    LoadDsl,
    SelectDsl
};
use diesel::expression::dsl::{any, max};
use diesel::pg::PgConnection;
use ruma_identifiers::{EventId, UserId};

use error::ApiError;
use schema::{presence_events, presence_list};

/// A Matrix presence stream, not saved yet.
#[derive(Debug, Clone, Insertable)]
#[table_name = "presence_events"]
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
    pub fn insert(
        connection: &PgConnection,
        event_id: &EventId,
        user_id: &UserId,
        presence: &String
    ) -> Result<PresenceStreamEvent, ApiError> {
        let new_event = NewPresenceStreamEvent {
            event_id: event_id.clone(),
            user_id: user_id.clone(),
            presence: presence.clone()
        };
        insert(&new_event)
            .into(presence_events::table)
            .get_result(connection)
            .map_err(ApiError::from)
    }

    pub fn find_for_presence_list_by_uid(
        connection: &PgConnection,
        user_id: &UserId,
        since: Option<i64>
    ) -> Result<Vec<PresenceStreamEvent>, ApiError> {
        let users = presence_list::table
            .filter(presence_list::user_id.eq(user_id))
            .select(presence_list::observed_user_id);

        if let Some(since) = since {
            let ordering = presence_events::table
                .filter(presence_events::user_id.eq(any(&users)))
                .filter(presence_events::ordering.gt(since))
                .group_by(presence_events::user_id)
                .select(max(presence_events::ordering));

            presence_events::table
                .filter(presence_events::ordering.eq(any(&ordering)))
                .get_results(connection)
                .map_err(ApiError::from)
        } else {
            let ordering = presence_events::table
                .filter(presence_events::user_id.eq(any(&users)))
                .group_by(presence_events::user_id)
                .select(max(presence_events::ordering));

            presence_events::table
                .filter(presence_events::ordering.eq(any(&ordering)))
                .get_results(connection)
                .map_err(ApiError::from)
        }
    }
}
