#[macro_use]
extern crate rocket;

use rocket::http::Status;
use rocket::request::FromRequest;
use rocket::request::Outcome;
use rocket::serde::{json::Json, Serialize};
use rocket::{
    fairing::{Fairing, Info, Kind},
    Data, Request, Response,
};
use tracing::{info_span, Span};
use uuid::Uuid;

#[derive(Clone)]
pub struct RequestId<T = Uuid>(pub T);

impl Default for RequestId {
    fn default() -> Self {
        Self(Uuid::new_v4())
    }
}

// Allows a route to access the span
#[rocket::async_trait]
impl<'r> FromRequest<'r> for RequestId {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, ()> {
        match &*request.local_cache(|| RequestId::<Option<Uuid>>(None)) {
            RequestId(Some(request_id)) => Outcome::Success(RequestId(request_id.to_owned())),
            RequestId(None) => Outcome::Failure((Status::InternalServerError, ())),
        }
    }
}

#[derive(Clone)]
pub struct TracingSpan<T = Span>(T);

struct TracingFairing;

#[rocket::async_trait]
impl Fairing for TracingFairing {
    fn info(&self) -> Info {
        Info {
            name: "Tracing Fairing",
            kind: Kind::Request | Kind::Response,
        }
    }
    async fn on_request(&self, req: &mut Request<'_>, _data: &mut Data<'_>) {
        let request_id = RequestId::default();
        req.local_cache(|| RequestId(Some(request_id.0.to_owned())));

        let user_agent = req.headers().get_one("User-Agent").unwrap_or("");

        let span = info_span!(
            "request",
            method = %req.method(),
            uri = %req.uri().path(),
            user_agent,
            status_code = tracing::field::Empty,
            request_id=%request_id.0
        );

        req.local_cache(|| TracingSpan::<Option<Span>>(Some(span)));
    }

    async fn on_response<'r>(&self, req: &'r Request<'_>, res: &mut Response<'r>) {
        let span = req.local_cache(|| TracingSpan::<Option<Span>>(None));
        if let Some(span) = &span.0 {
            span.record("status_code", &res.status().code);
        }
    }
}

// Allows a route to access the span
#[rocket::async_trait]
impl<'r> FromRequest<'r> for TracingSpan {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, ()> {
        match &*request.local_cache(|| TracingSpan::<Option<Span>>(None)) {
            TracingSpan(Some(span)) => Outcome::Success(TracingSpan(span.to_owned())),
            TracingSpan(None) => Outcome::Failure((Status::InternalServerError, ())),
        }
    }
}

#[derive(Serialize, Debug)]
pub struct OutputData<'a> {
    pub message: &'a str,
    pub request_id: String,
}

#[get("/abc")]
pub async fn abc<'a>(
    span: TracingSpan,
    request_id: RequestId,
) -> Result<Json<OutputData<'a>>, Status> {
    let mock_data = OutputData {
        message: "Hello World",
        request_id: request_id.0.to_string(),
    };
    span.0.record(
        "output",
        &serde_json::to_string(&mock_data).unwrap().as_str(),
    );
    Ok(Json(mock_data))
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/", routes![abc])
        .attach(TracingFairing)
}
