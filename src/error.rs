use std::fmt;
use std::time::Duration;

use reqwest::StatusCode;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ErrorKind {
    Configuration,
    InvalidRequest,
    Transport,
    Timeout,
    HttpApi,
    Decode,
    StreamProtocol,
    UnexpectedEof,
    LimitExceeded,
    Accumulation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum TimeoutStage {
    ResponseHeaders,
    ResponseBody,
    Operation,
}

/// Bounded provider error information.
#[derive(Clone)]
pub struct ApiError {
    pub status: StatusCode,
    pub error_type: Option<String>,
    pub code: Option<String>,
    pub request_id: Option<String>,
    pub retry_after: Option<Duration>,
    body: String,
    body_truncated: bool,
}

impl ApiError {
    pub(crate) fn new(
        status: StatusCode,
        error_type: Option<String>,
        code: Option<String>,
        request_id: Option<String>,
        retry_after: Option<Duration>,
        body: String,
        body_truncated: bool,
    ) -> Self {
        Self {
            status,
            error_type,
            code,
            request_id,
            retry_after,
            body,
            body_truncated,
        }
    }

    /// Access the bounded raw body explicitly. It may contain submitted data.
    pub fn diagnostic_body(&self) -> &str {
        &self.body
    }

    pub fn body_truncated(&self) -> bool {
        self.body_truncated
    }
}

impl fmt::Debug for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ApiError")
            .field("status", &self.status)
            .field("error_type", &self.error_type)
            .field("code", &self.code)
            .field("request_id", &self.request_id)
            .field("retry_after", &self.retry_after)
            .field("body_truncated", &self.body_truncated)
            .finish_non_exhaustive()
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Mistral API returned HTTP {}", self.status)?;
        if let Some(kind) = &self.error_type {
            write!(f, " ({kind})")?;
        }
        if let Some(request_id) = &self.request_id {
            write!(f, ", request {request_id}")?;
        }
        Ok(())
    }
}

impl std::error::Error for ApiError {}

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    #[error("invalid client configuration: {0}")]
    Configuration(String),
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    #[error("HTTP transport failed")]
    Transport(#[source] reqwest::Error),
    #[error("request timed out during {stage:?}")]
    Timeout { stage: TimeoutStage },
    #[error(transparent)]
    HttpApi(#[from] ApiError),
    #[error("failed to decode provider response: {0}")]
    Decode(String),
    #[error("invalid event stream: {0}")]
    StreamProtocol(String),
    #[error("event stream ended before the provider completion marker")]
    UnexpectedEof,
    #[error("{resource} exceeded configured limit of {limit}")]
    LimitExceeded {
        resource: &'static str,
        limit: usize,
    },
    #[error("cannot assemble streamed completion: {0}")]
    Accumulation(String),
}

impl Error {
    pub fn kind(&self) -> ErrorKind {
        match self {
            Self::Configuration(_) => ErrorKind::Configuration,
            Self::InvalidRequest(_) => ErrorKind::InvalidRequest,
            Self::Transport(_) => ErrorKind::Transport,
            Self::Timeout { .. } => ErrorKind::Timeout,
            Self::HttpApi(_) => ErrorKind::HttpApi,
            Self::Decode(_) => ErrorKind::Decode,
            Self::StreamProtocol(_) => ErrorKind::StreamProtocol,
            Self::UnexpectedEof => ErrorKind::UnexpectedEof,
            Self::LimitExceeded { .. } => ErrorKind::LimitExceeded,
            Self::Accumulation(_) => ErrorKind::Accumulation,
        }
    }
}
