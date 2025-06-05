use axum::{
    Json,
    extract::rejection::{JsonRejection, PathRejection, QueryRejection},
    response::IntoResponse,
};
use colored::Colorize;
use rquest::{Response, StatusCode, header::InvalidHeaderValue};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use snafu::Location;
use std::fmt::Display;
use strum::IntoStaticStr;
use tokio::sync::oneshot;
use tracing::{debug, error};

use crate::{
    config::Reason,
    services::{cookie_manager::CookieEvent, key_manager::KeyEvent},
    types::claude_message::Message,
};

#[derive(Debug, IntoStaticStr, snafu::Snafu)]
#[snafu(visibility(pub(crate)))]
#[strum(serialize_all = "snake_case")]
pub enum ClewdrError {
    #[snafu(display("Invalid URI: {}", uri))]
    InvalidUri {
        uri: String,
        source: http::uri::InvalidUri,
    },
    #[snafu(display("YuOAuth2 error: {}", source))]
    #[snafu(context(false))]
    YuOAuth2Error { source: yup_oauth2::Error },
    #[snafu(display("Empty choices"))]
    EmptyChoices,
    #[snafu(display("JSON error: {}", source))]
    #[snafu(context(false))]
    JsonError { source: serde_json::Error },
    #[snafu(transparent)]
    PathRejection { source: PathRejection },
    #[snafu(transparent)]
    QueryRejection { source: QueryRejection },
    #[snafu(display("Cache found"))]
    CacheFound { res: Box<axum::response::Response> },
    #[snafu(display("Test Message"))]
    TestMessage,
    #[snafu(display("FmtError: {}", source))]
    #[snafu(context(false))]
    FmtError {
        #[snafu(implicit)]
        loc: Location,
        source: std::fmt::Error,
    },
    #[snafu(display("Invalid header value: {}", source))]
    #[snafu(context(false))]
    InvalidHeaderValue { source: InvalidHeaderValue },
    #[snafu(display("Bad request: {}", msg))]
    BadRequest { msg: &'static str },
    #[snafu(display("Pad text too short"))]
    PadtxtTooShort,
    #[snafu(display("Key send error: {}", source))]
    #[snafu(context(false))]
    KeySendError {
        source: tokio::sync::mpsc::error::SendError<KeyEvent>,
    },
    #[snafu(display("Cookie send error: {}", source))]
    #[snafu(context(false))]
    CookieSendError {
        source: tokio::sync::mpsc::error::SendError<CookieEvent>,
    },
    #[snafu(display("Retries exceeded"))]
    TooManyRetries,
    #[snafu(display("EventSource error: {}", source))]
    #[snafu(context(false))]
    EventSourceAxumError {
        source: eventsource_stream::EventStreamError<axum::Error>,
    },
    #[snafu(context(false))]
    EventSourceRquestError {
        source: eventsource_stream::EventStreamError<rquest::Error>,
    },
    #[snafu(display("Zip error: {}", source))]
    #[snafu(context(false))]
    ZipError { source: zip::result::ZipError },
    #[snafu(display("Asset Error: {}", msg))]
    AssetError { msg: String },
    #[snafu(display("Invalid version: {}", version))]
    InvalidVersion { version: String },
    #[snafu(display("ParseInt error: {}", source))]
    #[snafu(context(false))]
    ParseIntError { source: std::num::ParseIntError },
    #[snafu(display("Cookie dispatch error: {}", source))]
    #[snafu(context(false))]
    CookieDispatchError { source: oneshot::error::RecvError },
    #[snafu(display("No cookie available"))]
    NoCookieAvailable,
    #[snafu(display("No key available"))]
    NoKeyAvailable,
    #[snafu(display("Invalid Cookie: {}", reason))]
    #[snafu(context(false))]
    InvalidCookie {
        #[snafu(source)]
        reason: Reason,
    },
    #[snafu(display("Failed to parse TOML: {}", source))]
    #[snafu(context(false))]
    TomlDeError { source: toml::de::Error },
    #[snafu(transparent)]
    TomlSeError { source: toml::ser::Error },
    #[snafu(transparent)]
    JsonRejection { source: JsonRejection },
    #[snafu(display("Rquest error: {}, source: {}", msg, source))]
    RquestError {
        msg: &'static str,
        source: rquest::Error,
    },
    #[snafu(display("UTF-8 error: {}", source))]
    #[snafu(context(false))]
    UTF8Error {
        #[snafu(implicit)]
        loc: Location,
        source: std::string::FromUtf8Error,
    },
    #[snafu(display("Http error: code: {}, body: {}", code.to_string().red(), inner.to_string()))]
    ClaudeHttpError {
        code: StatusCode,
        inner: ClaudeErrorBody,
    },
    #[snafu(display("Http error: code: {}, body: {}", code.to_string().red(), serde_json::to_string_pretty(&inner).unwrap_or_default()))]
    GeminiHttpError { code: StatusCode, inner: Value },
    #[snafu(display("Unexpected None: {}", msg))]
    UnexpectedNone { msg: &'static str },
    #[snafu(display("IO error: {}", source))]
    #[snafu(context(false))]
    IoError {
        #[snafu(implicit)]
        loc: Location,
        source: std::io::Error,
    },
    #[snafu(display("{}", msg))]
    PathNotFound { msg: String },
    #[snafu(display("Invalid timestamp: {}", timestamp))]
    TimestampError { timestamp: i64 },
    #[snafu(display("Key/Password Invalid"))]
    InvalidKey,
}

impl IntoResponse for ClewdrError {
    fn into_response(self) -> axum::response::Response {
        let (status, msg) = match self {
            ClewdrError::InvalidUri { .. } => (StatusCode::BAD_REQUEST, json!(self.to_string())),
            ClewdrError::YuOAuth2Error { .. } => {
                (StatusCode::UNAUTHORIZED, json!(self.to_string()))
            }
            ClewdrError::PathRejection { ref source } => {
                (source.status(), json!(source.body_text()))
            }
            ClewdrError::QueryRejection { ref source } => {
                (source.status(), json!(source.body_text()))
            }
            ClewdrError::ClaudeHttpError { code, inner } => {
                return (code, Json(ClaudeError { error: inner })).into_response();
            }
            ClewdrError::GeminiHttpError { code, inner } => {
                return (code, Json(inner)).into_response();
            }
            ClewdrError::CacheFound { res } => return (StatusCode::OK, *res).into_response(),
            ClewdrError::TestMessage => {
                return (
                    StatusCode::OK,
                    Json(Message::from(
                        "Claude Reverse Proxy is working, please send a real message.",
                    )),
                )
                    .into_response();
            }
            ClewdrError::JsonRejection { ref source } => {
                (source.status(), json!(source.body_text()))
            }
            ClewdrError::PadtxtTooShort => (StatusCode::BAD_REQUEST, json!(self.to_string())),
            ClewdrError::TooManyRetries => (StatusCode::GATEWAY_TIMEOUT, json!(self.to_string())),
            ClewdrError::InvalidCookie { .. } => (StatusCode::BAD_REQUEST, json!(self.to_string())),
            ClewdrError::PathNotFound { .. } => (StatusCode::NOT_FOUND, json!(self.to_string())),
            ClewdrError::InvalidKey => (StatusCode::UNAUTHORIZED, json!(self.to_string())),
            ClewdrError::BadRequest { .. } => (StatusCode::BAD_REQUEST, json!(self.to_string())),
            ClewdrError::InvalidHeaderValue { .. } => {
                (StatusCode::BAD_REQUEST, json!(self.to_string()))
            }
            ClewdrError::EmptyChoices => (StatusCode::NO_CONTENT, json!(self.to_string())),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, json!(self.to_string())),
        };
        let err = ClaudeError {
            error: ClaudeErrorBody {
                message: msg,
                r#type: <&'static str>::from(self).into(),
                code: Some(status.as_u16()),
            },
        };
        (status, Json(err)).into_response()
    }
}

/// HTTP error response
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ClaudeError {
    pub error: ClaudeErrorBody,
}

/// Inner HTTP error response
#[derive(Debug, Serialize, Clone)]
pub struct ClaudeErrorBody {
    pub message: Value,
    pub r#type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<u16>,
}

/// Raw Inner HTTP error response
#[derive(Debug, Deserialize)]
struct RawBody {
    pub message: String,
    pub r#type: String,
}

impl<'de> Deserialize<'de> for ClaudeErrorBody {
    /// when message is a json string, try parse it as a object
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = RawBody::deserialize(deserializer)?;
        if let Ok(message) = serde_json::from_str::<Value>(&raw.message) {
            return Ok(ClaudeErrorBody {
                message,
                r#type: raw.r#type,
                code: None,
            });
        }
        Ok(ClaudeErrorBody {
            message: json!(raw.message),
            r#type: raw.r#type,
            code: None,
        })
    }
}

impl Display for ClaudeErrorBody {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        serde_json::to_string_pretty(self)
            .map_err(|_| std::fmt::Error)?
            .fmt(f)
    }
}

pub trait CheckClaudeErr
where
    Self: Sized,
{
    fn check_claude(self) -> impl Future<Output = Result<Self, ClewdrError>>;
}

pub trait CheckGeminiErr
where
    Self: Sized,
{
    fn check_gemini(self) -> impl Future<Output = Result<Self, ClewdrError>>;
}

impl CheckGeminiErr for Response {
    async fn check_gemini(self) -> Result<Self, ClewdrError> {
        let status = self.status();
        if status.is_success() {
            return Ok(self);
        }
        // else just return OtherHttpError
        let text = match self.text().await {
            Ok(text) => text,
            Err(err) => {
                let error = json!({
                    "message": err.to_string(),
                    "status": "error_get_error_body",
                    "code": status.as_u16()
                });
                return Err(ClewdrError::GeminiHttpError {
                    code: status,
                    inner: error,
                });
            }
        };
        let Ok(error) = serde_json::from_str::<Value>(&text) else {
            let error = json!({
                "message": format!("Unknown error: {}", text),
                "status": "error_parse_error_body",
                "code": Some(status.as_u16()),
            });
            return Err(ClewdrError::GeminiHttpError {
                code: status,
                inner: error,
            });
        };
        Err(ClewdrError::GeminiHttpError {
            code: status,
            inner: error,
        })
    }
}

impl CheckClaudeErr for Response {
    /// Checks response from Claude Web API for errors
    /// Validates HTTP status codes and parses error messages from responses
    ///
    /// # Arguments
    /// * `res` - The HTTP response to check
    ///
    /// # Returns
    /// * `Ok(Response)` if the request was successful
    /// * `Err(ClewdrError)` if the request failed, with details about the failure
    async fn check_claude(self) -> Result<Self, ClewdrError> {
        let status = self.status();
        if status.is_success() {
            return Ok(self);
        }
        debug!("Error response status: {}", status);
        if status == 302 {
            // blocked by cloudflare
            let error = ClaudeErrorBody {
                message: json!("Blocked, check your IP address"),
                r#type: "error".to_string(),
                code: Some(status.as_u16()),
            };
            return Err(ClewdrError::ClaudeHttpError {
                code: status,
                inner: error,
            });
        }
        let text = match self.text().await {
            Ok(text) => text,
            Err(err) => {
                let error = ClaudeErrorBody {
                    message: json!(err.to_string()),
                    r#type: "error_get_error_body".to_string(),
                    code: Some(status.as_u16()),
                };
                return Err(ClewdrError::ClaudeHttpError {
                    code: status,
                    inner: error,
                });
            }
        };
        let Ok(err) = serde_json::from_str::<ClaudeError>(&text) else {
            let error = ClaudeErrorBody {
                message: format!("Unknown error: {}", text).into(),
                r#type: "error_parse_error_body".to_string(),
                code: Some(status.as_u16()),
            };
            return Err(ClewdrError::ClaudeHttpError {
                code: status,
                inner: error,
            });
        };
        if status == 400 && err.error.message == json!("This organization has been disabled.") {
            // account disabled
            return Err(Reason::Disabled.into());
        }
        let inner_error = err.error;
        // check if the error is a rate limit error
        if status == 429 {
            // get the reset time from the error message
            if let Some(time) = inner_error.message["resetsAt"].as_i64() {
                let reset_time = chrono::DateTime::from_timestamp(time, 0)
                    .ok_or(ClewdrError::TimestampError { timestamp: time })?
                    .to_utc();
                let now = chrono::Utc::now();
                let diff = reset_time - now;
                let hours = diff.num_hours();
                error!("Rate limit exceeded, expires in {} hours", hours);
                return Err(ClewdrError::InvalidCookie {
                    reason: Reason::TooManyRequest(time),
                });
            }
        }
        Err(ClewdrError::ClaudeHttpError {
            code: status,
            inner: inner_error,
        })
    }
}
