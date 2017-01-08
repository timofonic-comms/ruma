//! Storage and querying of presence lists.

use std::cmp;
use std::time::SystemTime;

use diesel::{
    delete,
    insert,
    Connection,
    ExpressionMethods,
    ExecuteDsl,
    FilterDsl,
};
use diesel::expression::dsl::any;
use diesel::pg::PgConnection;
use ruma_events::EventType;
use ruma_events::presence::{PresenceEvent, PresenceEventContent, PresenceState};
use ruma_identifiers::UserId;

use error::ApiError;
use models::presence_status::PresenceStatus;
use models::presence_event::PresenceStreamEvent;
use models::user::User;
use schema::presence_list;

/// A Matrix presence list.
#[derive(Debug, Clone, Insertable, Queryable)]
#[table_name = "presence_list"]
pub struct PresenceList {
    /// Initiator.
    pub user_id: UserId,
    /// Observed user.
    pub observed_user_id: UserId,
}

impl PresenceList {
    /// Combines creations and deletions of multiple presence list entries.
    pub fn update(
        connection: &PgConnection,
        user_id: &UserId,
        invite: &Vec<UserId>,
        drop: Vec<UserId>
    ) -> Result<(), ApiError> {
        connection.transaction::<(()), ApiError, _>(|| {
            let missing_user_ids = User::find_missing_users(
                connection,
                invite
            )?;
            if !missing_user_ids.is_empty() {
                return Err(
                    ApiError::bad_json(format!(
                        "Unknown users in invite list: {}",
                        &missing_user_ids
                            .iter()
                            .map(|user_id| user_id.to_string())
                            .collect::<Vec<String>>()
                            .join(", ")
                    ))
                )
            }


            let mut invites: Vec<PresenceList> = Vec::new();
            for observed_user in invite.clone() {
                invites.push(PresenceList {
                    user_id: user_id.clone(),
                    observed_user_id: observed_user,
                });
            }
            insert(&invites)
                .into(presence_list::table)
                .execute(connection)
                .map_err(ApiError::from)?;

            let missing_user_ids = User::find_missing_users(
                connection,
                &drop
            )?;
            if !missing_user_ids.is_empty() {
                return Err(
                    ApiError::bad_json(format!(
                        "Unknown users in drop list: {}",
                        &missing_user_ids
                            .iter()
                            .map(|user_id| user_id.to_string())
                            .collect::<Vec<String>>()
                            .join(", ")
                    ))
                )
            }

            let drop = presence_list::table
                .filter(presence_list::user_id.eq(user_id))
                .filter(presence_list::observed_user_id.eq(any(drop)));
            delete(drop)
                .execute(connection)
                .map_err(ApiError::from)?;
            Ok(())
        }).map_err(ApiError::from)
    }

    /// Return `PresenceEvent`'s for given `UserId`.
    pub fn find_events_by_uid(
        connection: &PgConnection,
        user_id: &UserId,
        since: Option<i64>
    ) -> Result<(i64, Vec<PresenceEvent>), ApiError> {
        let mut max_ordering = -1;

        let stream_events= PresenceStreamEvent::find_events_by_uid(
            connection,
            user_id,
            since
        )?;

        let mut events = Vec::new();
        let now = SystemTime::now();
        for stream_event in stream_events {
            max_ordering = cmp::max(stream_event.ordering, max_ordering);

            let presence_state: PresenceState = stream_event.presence.parse().expect("Something wrong with the database!");
            let last_active_ago = PresenceStatus::calculate_last_active_ago(stream_event.created_at, now)?;

            events.push(PresenceEvent {
                content: PresenceEventContent {
                    avatar_url: stream_event.avatar_url,
                    currently_active: PresenceState::Online == presence_state,
                    displayname: stream_event.displayname,
                    last_active_ago: Some(last_active_ago),
                    presence: presence_state,
                    user_id: stream_event.user_id,
                },
                event_type: EventType::Presence,
                event_id: stream_event.event_id
            });
        }

        Ok((max_ordering, events))
    }
}
