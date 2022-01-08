#[macro_use]
extern crate rocket;

#[macro_use]
extern crate tracing;

use rocket::http::Status;
use rocket::request::FromRequest;
use rocket::request::Outcome;
use rocket::serde::{json::Json, Serialize};
use rocket::{
    fairing::{Fairing, Info, Kind},
    Data, Request, Response,
};

use tracing::{info_span, Span};
use tracing_log::LogTracer;

use tracing_subscriber::Layer;
use tracing_subscriber::{registry::LookupSpan, EnvFilter};
use uuid::Uuid;
use yansi::Paint;

// Spans

#[derive(Clone, Debug)]
pub struct RequestId<T = Uuid>(pub T);

impl Default for RequestId {
    fn default() -> Self {
        Self(Uuid::new_v4())
    }
}

// Allows a route to access the request id
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
            otel.name=%format!("{} {}", req.method(), req.uri().path()),
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
    let entered = span.0.enter();
    info!("Hello World");

    let mock_data = OutputData {
        message: "Hello World",
        request_id: request_id.0.to_string(),
    };
    span.0.record(
        "output",
        &serde_json::to_string(&mock_data).unwrap().as_str(),
    );
    drop(entered);
    Ok(Json(mock_data))
}

// Logging

use tracing_subscriber::field::MakeExt;

pub fn logging_layer<S>() -> impl Layer<S>
where
    S: tracing::Subscriber,
    S: for<'span> LookupSpan<'span>,
{
    let field_format = tracing_subscriber::fmt::format::debug_fn(|writer, field, value| {
        // We'll format the field name and value separated with a colon.
        let name = field.name();
        if name == "message" {
            write!(writer, "{:?}", Paint::new(value).bold())
        } else {
            write!(writer, "{}: {:?}", field, Paint::default(value).bold())
        }
    })
    .delimited(", ")
    .display_messages();

    tracing_subscriber::fmt::layer()
        .fmt_fields(field_format)
        // Configure the formatter to use `print!` rather than
        // `stdout().write_str(...)`, so that logs are captured by libtest's test
        // capturing.
        .with_test_writer()
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum LogLevel {
    /// Only shows errors and warnings: `"critical"`.
    Critical,
    /// Shows errors, warnings, and some informational messages that are likely
    /// to be relevant when troubleshooting such as configuration: `"support"`.
    Support,
    /// Shows everything except debug and trace information: `"normal"`.
    Normal,
    /// Shows everything: `"debug"`.
    Debug,
    /// Shows nothing: "`"off"`".
    Off,
}

impl From<&str> for LogLevel {
    fn from(s: &str) -> Self {
        return match &*s.to_ascii_lowercase() {
            "critical" => LogLevel::Critical,
            "support" => LogLevel::Support,
            "normal" => LogLevel::Normal,
            "debug" => LogLevel::Debug,
            "off" => LogLevel::Off,
            _ => panic!("a log level (off, debug, normal, support, critical)"),
        };
    }
}

pub fn filter_layer(level: LogLevel) -> EnvFilter {
    let filter_str = match level {
        LogLevel::Critical => "warn,hyper=off,rustls=off",
        LogLevel::Support => "warn,rocket::support=info,hyper=off,rustls=off",
        LogLevel::Normal => "info,hyper=off,rustls=off",
        LogLevel::Debug => "trace",
        LogLevel::Off => "off",
    };

    tracing_subscriber::filter::EnvFilter::try_new(filter_str).expect("filter string must parse")
}

// Rocket setup

#[launch]
fn rocket() -> _ {
    use tracing_subscriber::prelude::*;

    Paint::disable();

    LogTracer::init().expect("Unable to setup log tracer!");

    tracing::subscriber::set_global_default(
        tracing_subscriber::registry()
            .with(logging_layer())
            .with(filter_layer(LogLevel::from(
                std::env::var("LOG_LEVEL")
                    .unwrap_or_else(|_| "normal".to_string())
                    .as_str(),
            ))),
    )
    .unwrap();

    rocket::build()
        .mount("/", routes![abc])
        .attach(TracingFairing)
}
