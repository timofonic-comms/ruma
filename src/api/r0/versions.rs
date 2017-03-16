//! Endpoints for information about supported versions of the Matrix spec.

use iron::{Chain, Handler, IronResult, Request, Response, status};

use middleware::MiddlewareChain;
use modifier::SerializableResponse;

/// The `/versions` endpoint.
pub struct Versions;

/// Endpoint's response.
#[derive(Serialize)]
struct VersionsResponse {
    versions: Vec<&'static str>,
}

middleware_chain!(Versions);

impl VersionsResponse {
    /// Returns the list of supported `Versions` of the Matrix spec.
    pub fn supported() -> Self {
        VersionsResponse {
            versions: vec![
                "r0.2.0"
            ]
        }
    }
}

impl Handler for Versions {
    fn handle(&self, _request: &mut Request) -> IronResult<Response> {
        Ok(Response::with((status::Ok, SerializableResponse(VersionsResponse::supported()))))
    }
}
