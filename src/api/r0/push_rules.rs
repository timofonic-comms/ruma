//! Matrix push rule set.

use iron::status::Status;
use iron::{Chain, Handler, IronResult, Request, Response};

use middleware::{AccessTokenAuth, MiddlewareChain};
use models::user::User;
use modifier::SerializableResponse;

/// The GET `/pushrules` endpoint.
pub struct GetPushRules;

#[derive(Clone, Debug, Serialize)]
pub struct GetPushRulesResponse {
    /// The global ruleset.
    pub global: RuleSet
}

#[derive(Clone, Debug, Serialize)]
pub struct RuleSet {
    pub content: Vec<PushRule>,
    #[serde(rename="override")]
    pub override_rule: Vec<PushRule>,
    pub room: Vec<PushRule>,
    pub sender: Vec<PushRule>,
    pub underride: Vec<PushRule>,
}

#[derive(Clone, Debug, Serialize)]
pub struct PushRule {
    pub actions: String,
    pub default: bool,
    pub enabled: bool,
    pub rule_id: String,
}

middleware_chain!(GetPushRules, [AccessTokenAuth]);

impl Handler for GetPushRules {
    fn handle(&self, request: &mut Request) -> IronResult<Response> {
        let _ = request.extensions.get::<User>()
            .expect("AccessTokenAuth should ensure a user").clone();

        let response = GetPushRulesResponse {
            global: RuleSet {
                content: Vec::new(),
                override_rule: Vec::new(),
                room: Vec::new(),
                sender: Vec::new(),
                underride: Vec::new(),
            }
        };

        Ok(Response::with((Status::Ok, SerializableResponse(response))))
    }
}
