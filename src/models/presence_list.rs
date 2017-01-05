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
    LoadDsl,
};
use diesel::expression::dsl::any;
use diesel::pg::PgConnection;
use ruma_events::EventType;
use ruma_events::presence::{PresenceEvent, PresenceEventContent, PresenceState};
use ruma_identifiers::UserId;
use serde_json::{from_value, Value};

use error::ApiError;
use models::presence_status::PresenceStatus;
use models::presence_event::PresenceStreamEvent;
use models::profile::Profile;
use models::user::User;
use schema::{presence_list, users};

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
            let users: Vec<User> = users::table
                        .filter(users::id.eq(any(invite)))
                        .get_results(connection)
                        .map_err(ApiError::from)?;
            if users.len() != invite.len() {
                return Err(ApiError::invalid_param("invite", "Users does not exists!"));
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

        let stream_events= PresenceStreamEvent::find_for_presence_list_by_uid(
            connection,
            user_id,
            since
        )?;

        let profiles = Profile::find_for_presence_list_by_uid(
            connection,
            user_id
        )?;

        let mut events = Vec::new();
        let now = SystemTime::now();
        for stream_event in stream_events {
            max_ordering = cmp::max(stream_event.ordering, max_ordering);

            let profile: Option<&Profile> = profiles.iter().filter(|profile| profile.id == stream_event.user_id).next();
            let mut avatar_url = None;
            let mut displayname = None;
            if let Some(ref profile) = profile {
                avatar_url = profile.avatar_url.clone();
                displayname = profile.displayname.clone();
            }

            let presence_state: PresenceState = from_value(Value::String(stream_event.presence)).map_err(ApiError::from)?;
            let last_active_ago = PresenceStatus::calculate_last_active_ago(stream_event.created_at, now)?;
            let currently_active = last_active_ago < (5 * 60 * 1000) && presence_state == PresenceState::Online;

            events.push(PresenceEvent {
                content: PresenceEventContent {
                    avatar_url: avatar_url,
                    currently_active: currently_active,
                    displayname: displayname,
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
