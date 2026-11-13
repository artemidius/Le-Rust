use std::fmt;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use futures_util::StreamExt;
use reqwest::Method;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue, RETRY_AFTER};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;
use url::Url;

use crate::chat::Chat;
use crate::error::{ApiError, Error, Result, TimeoutStage};
use crate::fim::Fim;
use crate::models::Models;
use crate::stream::CompletionStream;
use crate::types::{ApiResponse, ResponseMeta};

const DEFAULT_BASE_URL: &str = "https://api.mistral.ai/v1/";

/// Configurable ceilings for untrusted provider responses and stream state.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct Limits {
    pub response_body_bytes: usize,
    pub error_body_bytes: usize,
    pub sse_line_bytes: usize,
    pub sse_event_bytes: usize,
    pub sse_queued_events: usize,
    pub accumulated_completion_bytes: usize,
    pub tool_arguments_bytes: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            response_body_bytes: 16 * 1024 * 1024,
            error_body_bytes: 64 * 1024,
            sse_line_bytes: 1024 * 1024,
            sse_event_bytes: 1024 * 1024,
            sse_queued_events: 256,
            accumulated_completion_bytes: 16 * 1024 * 1024,
            tool_arguments_bytes: 1024 * 1024,
        }
    }
}

/// Options that may narrow client defaults for one operation.
#[derive(Debug, Clone, Copy, Default)]
#[non_exhaustive]
pub struct RequestOptions {
    pub header_timeout: Option<Duration>,
    pub operation_timeout: Option<Duration>,
    pub response_body_bytes: Option<usize>,
}

#[derive(Clone)]
/// Cloneable asynchronous client sharing one connection pool.
pub struct Client {
    pub(crate) inner: Arc<ClientInner>,
}

pub(crate) struct ClientInner {
    pub http: reqwest::Client,
    pub base_url: Url,
    pub authorization: HeaderValue,
    pub header_timeout: Duration,
    pub operation_timeout: Duration,
    pub limits: Limits,
}

impl fmt::Debug for Client {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Client")
            .field("base_url", &self.inner.base_url)
            .field("header_timeout", &self.inner.header_timeout)
            .field("operation_timeout", &self.inner.operation_timeout)
            .field("limits", &self.inner.limits)
            .field("authorization", &"[REDACTED]")
            .finish()
    }
}

#[derive(Clone)]
/// Builder for authentication, transport deadlines, and response limits.
pub struct ClientBuilder {
    api_key: Option<String>,
    base_url: String,
    connect_timeout: Duration,
    read_timeout: Duration,
    header_timeout: Duration,
    operation_timeout: Duration,
    limits: Limits,
    allow_insecure_loopback: bool,
}

impl fmt::Debug for ClientBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClientBuilder")
            .field("api_key", &self.api_key.as_ref().map(|_| "[REDACTED]"))
            .field("base_url", &self.base_url)
            .field("connect_timeout", &self.connect_timeout)
            .field("read_timeout", &self.read_timeout)
            .field("header_timeout", &self.header_timeout)
            .field("operation_timeout", &self.operation_timeout)
            .field("limits", &self.limits)
            .field("allow_insecure_loopback", &self.allow_insecure_loopback)
            .finish()
    }
}

impl Default for ClientBuilder {
    fn default() -> Self {
        Self {
            api_key: None,
            base_url: DEFAULT_BASE_URL.into(),
            connect_timeout: Duration::from_secs(10),
            read_timeout: Duration::from_secs(60),
            header_timeout: Duration::from_secs(60),
            operation_timeout: Duration::from_secs(10 * 60),
            limits: Limits::default(),
            allow_insecure_loopback: false,
        }
    }
}

impl ClientBuilder {
    pub fn api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    pub fn base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    /// Permit plain HTTP only for a loopback test/development endpoint.
    pub fn allow_insecure_loopback(mut self, allow: bool) -> Self {
        self.allow_insecure_loopback = allow;
        self
    }

    pub fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = timeout;
        self
    }

    pub fn read_timeout(mut self, timeout: Duration) -> Self {
        self.read_timeout = timeout;
        self
    }

    pub fn header_timeout(mut self, timeout: Duration) -> Self {
        self.header_timeout = timeout;
        self
    }

    pub fn operation_timeout(mut self, timeout: Duration) -> Self {
        self.operation_timeout = timeout;
        self
    }

    pub fn limits(mut self, limits: Limits) -> Self {
        self.limits = limits;
        self
    }

    pub fn build(self) -> Result<Client> {
        let api_key = self
            .api_key
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| Error::Configuration("API key is required".into()))?;
        validate_duration("connect timeout", self.connect_timeout)?;
        validate_duration("read timeout", self.read_timeout)?;
        validate_duration("header timeout", self.header_timeout)?;
        validate_duration("operation timeout", self.operation_timeout)?;
        validate_limits(self.limits)?;

        let mut base_url = Url::parse(&self.base_url)
            .map_err(|err| Error::Configuration(format!("invalid base URL: {err}")))?;
        if !base_url.username().is_empty() || base_url.password().is_some() {
            return Err(Error::Configuration(
                "base URL must not contain credentials".into(),
            ));
        }
        if base_url.query().is_some() || base_url.fragment().is_some() {
            return Err(Error::Configuration(
                "base URL must not contain a query or fragment".into(),
            ));
        }
        let loopback = base_url
            .host_str()
            .is_some_and(|host| matches!(host, "localhost" | "127.0.0.1" | "::1"));
        if base_url.scheme() != "https" && !(self.allow_insecure_loopback && loopback) {
            return Err(Error::Configuration(
                "base URL must use HTTPS (plain HTTP is opt-in for loopback only)".into(),
            ));
        }
        if !base_url.path().ends_with('/') {
            base_url.set_path(&format!("{}/", base_url.path()));
        }

        let bearer = format!("Bearer {api_key}");
        let mut authorization = HeaderValue::from_str(&bearer)
            .map_err(|_| Error::Configuration("API key is not a valid header value".into()))?;
        authorization.set_sensitive(true);
        let http = reqwest::Client::builder()
            .connect_timeout(self.connect_timeout)
            .read_timeout(self.read_timeout)
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .user_agent(concat!("le-rust/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(Error::Transport)?;

        Ok(Client {
            inner: Arc::new(ClientInner {
                http,
                base_url,
                authorization,
                header_timeout: self.header_timeout,
                operation_timeout: self.operation_timeout,
                limits: self.limits,
            }),
        })
    }
}

impl Client {
    /// Start building a client without performing network I/O.
    pub fn builder() -> ClientBuilder {
        ClientBuilder::default()
    }

    /// Build a client from `MISTRAL_API_KEY` and all other defaults.
    pub fn from_env() -> Result<Self> {
        let api_key = std::env::var("MISTRAL_API_KEY")
            .map_err(|_| Error::Configuration("MISTRAL_API_KEY is not set".into()))?;
        Self::builder().api_key(api_key).build()
    }

    /// Access model discovery operations.
    pub fn models(&self) -> Models<'_> {
        Models { client: self }
    }

    /// Access native chat-completion operations.
    pub fn chat(&self) -> Chat<'_> {
        Chat { client: self }
    }

    /// Access native fill-in-the-middle operations.
    pub fn fim(&self) -> Fim<'_> {
        Fim { client: self }
    }

    pub(crate) fn url(&self, path: &str) -> Result<Url> {
        self.inner
            .base_url
            .join(path)
            .map_err(|err| Error::InvalidRequest(format!("invalid endpoint path: {err}")))
    }

    pub(crate) fn path_with_segment(&self, path: &str, segment: &str) -> Result<Url> {
        let mut url = self.url(path)?;
        url.path_segments_mut()
            .map_err(|_| Error::Configuration("base URL cannot contain path segments".into()))?
            .push(segment);
        Ok(url)
    }

    pub(crate) async fn get_json<T: DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, String)],
        options: RequestOptions,
    ) -> Result<ApiResponse<T>> {
        let url = self.url(path)?;
        self.get_json_url(url, query, options).await
    }

    pub(crate) async fn get_json_url<T: DeserializeOwned>(
        &self,
        url: Url,
        query: &[(&str, String)],
        options: RequestOptions,
    ) -> Result<ApiResponse<T>> {
        let request = self
            .request(Method::GET, url, options)
            .query(query)
            .build()
            .map_err(Error::Transport)?;
        self.execute_json(request, options).await
    }

    pub(crate) async fn post_json<T: DeserializeOwned, B: Serialize + ?Sized>(
        &self,
        path: &str,
        body: &B,
        options: RequestOptions,
    ) -> Result<ApiResponse<T>> {
        let request = self
            .request(Method::POST, self.url(path)?, options)
            .json(body)
            .build()
            .map_err(Error::Transport)?;
        self.execute_json(request, options).await
    }

    pub(crate) async fn post_stream<B: Serialize + ?Sized>(
        &self,
        path: &str,
        body: &B,
        options: RequestOptions,
    ) -> Result<CompletionStream> {
        let request = self
            .request(Method::POST, self.url(path)?, options)
            .header(reqwest::header::ACCEPT, "text/event-stream")
            .json(body)
            .build()
            .map_err(Error::Transport)?;
        let response = self.execute_headers(request, options).await?;
        if !response.status().is_success() {
            return Err(self.decode_api_error(response).await?.into());
        }
        let is_event_stream = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.split(';').next())
            .is_some_and(|value| value.trim().eq_ignore_ascii_case("text/event-stream"));
        if !is_event_stream {
            return Err(Error::StreamProtocol(
                "successful streaming response did not use text/event-stream".into(),
            ));
        }
        let meta = response_meta(response.headers());
        Ok(CompletionStream::new(
            response.bytes_stream(),
            meta,
            self.inner.limits,
        ))
    }

    fn request(
        &self,
        method: Method,
        url: Url,
        options: RequestOptions,
    ) -> reqwest::RequestBuilder {
        self.inner
            .http
            .request(method, url)
            .header(AUTHORIZATION, self.inner.authorization.clone())
            .timeout(
                options
                    .operation_timeout
                    .unwrap_or(self.inner.operation_timeout),
            )
    }

    async fn execute_json<T: DeserializeOwned>(
        &self,
        request: reqwest::Request,
        options: RequestOptions,
    ) -> Result<ApiResponse<T>> {
        let response = self.execute_headers(request, options).await?;
        if !response.status().is_success() {
            return Err(self.decode_api_error(response).await?.into());
        }
        let meta = response_meta(response.headers());
        let limit = options
            .response_body_bytes
            .unwrap_or(self.inner.limits.response_body_bytes);
        let bytes = collect_bounded(response, limit, "response body").await?;
        let data = serde_json::from_slice(&bytes).map_err(|err| Error::Decode(err.to_string()))?;
        Ok(ApiResponse::new(data, meta))
    }

    async fn execute_headers(
        &self,
        request: reqwest::Request,
        options: RequestOptions,
    ) -> Result<reqwest::Response> {
        #[cfg(feature = "tracing")]
        let trace = (
            request.method().as_str().to_owned(),
            request.url().path().to_owned(),
        );
        let header_timeout = options.header_timeout.unwrap_or(self.inner.header_timeout);
        match tokio::time::timeout(header_timeout, self.inner.http.execute(request)).await {
            Ok(Ok(response)) => {
                #[cfg(feature = "tracing")]
                tracing::debug!(
                    method = trace.0,
                    endpoint = trace.1,
                    status = response.status().as_u16(),
                    "Mistral API response headers received"
                );
                Ok(response)
            }
            Ok(Err(error)) if error.is_timeout() => Err(Error::Timeout {
                stage: TimeoutStage::ResponseHeaders,
            }),
            Ok(Err(error)) => Err(Error::Transport(error)),
            Err(_) => Err(Error::Timeout {
                stage: TimeoutStage::ResponseHeaders,
            }),
        }
    }

    async fn decode_api_error(&self, response: reqwest::Response) -> Result<ApiError> {
        let status = response.status();
        let meta = response_meta(response.headers());
        let limit = self.inner.limits.error_body_bytes;
        let (bytes, truncated) = collect_bounded_truncated(response, limit).await?;
        let body = String::from_utf8_lossy(&bytes).into_owned();
        let parsed: Option<Value> = serde_json::from_slice(&bytes).ok();
        let error_type = parsed
            .as_ref()
            .and_then(|value| value.get("type"))
            .and_then(Value::as_str)
            .map(str::to_owned);
        let code = parsed
            .as_ref()
            .and_then(|value| value.get("code"))
            .map(|value| match value {
                Value::String(value) => value.clone(),
                value => value.to_string(),
            });
        Ok(ApiError::new(
            status,
            error_type,
            code,
            meta.request_id,
            meta.retry_after,
            body,
            truncated,
        ))
    }
}

fn validate_duration(name: &str, value: Duration) -> Result<()> {
    if value.is_zero() {
        Err(Error::Configuration(format!("{name} must be positive")))
    } else {
        Ok(())
    }
}

fn validate_limits(limits: Limits) -> Result<()> {
    for (name, value) in [
        ("response body", limits.response_body_bytes),
        ("error body", limits.error_body_bytes),
        ("SSE line", limits.sse_line_bytes),
        ("SSE event", limits.sse_event_bytes),
        ("SSE queued events", limits.sse_queued_events),
        (
            "accumulated completion",
            limits.accumulated_completion_bytes,
        ),
        ("tool arguments", limits.tool_arguments_bytes),
    ] {
        if value == 0 {
            return Err(Error::Configuration(format!(
                "{name} byte limit must be positive"
            )));
        }
    }
    Ok(())
}

async fn collect_bounded(
    response: reqwest::Response,
    limit: usize,
    resource: &'static str,
) -> Result<Vec<u8>> {
    let (bytes, truncated) = collect_bounded_truncated(response, limit).await?;
    if truncated {
        Err(Error::LimitExceeded { resource, limit })
    } else {
        Ok(bytes)
    }
}

async fn collect_bounded_truncated(
    response: reqwest::Response,
    limit: usize,
) -> Result<(Vec<u8>, bool)> {
    let mut stream = response.bytes_stream();
    let mut output = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|error| {
            if error.is_timeout() {
                Error::Timeout {
                    stage: TimeoutStage::ResponseBody,
                }
            } else {
                Error::Transport(error)
            }
        })?;
        let remaining = limit.saturating_sub(output.len());
        if chunk.len() > remaining {
            output.extend_from_slice(&chunk[..remaining]);
            return Ok((output, true));
        }
        output.extend_from_slice(&chunk);
    }
    Ok((output, false))
}

fn response_meta(headers: &HeaderMap) -> ResponseMeta {
    let get = |names: &[&str]| {
        names.iter().find_map(|name| {
            headers
                .get(*name)
                .and_then(|value| value.to_str().ok())
                .map(str::to_owned)
        })
    };
    ResponseMeta {
        request_id: get(&["x-request-id", "request-id"]),
        retry_after: parse_retry_after(headers),
        rate_limit_limit: get(&["x-ratelimit-limit", "ratelimit-limit"]),
        rate_limit_remaining: get(&["x-ratelimit-remaining", "ratelimit-remaining"]),
        rate_limit_reset: get(&["x-ratelimit-reset", "ratelimit-reset"]),
    }
}

fn parse_retry_after(headers: &HeaderMap) -> Option<Duration> {
    if let Some(milliseconds) = headers
        .get("retry-after-ms")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
    {
        return Some(Duration::from_millis(milliseconds));
    }
    let value = headers.get(RETRY_AFTER)?.to_str().ok()?;
    if let Ok(seconds) = value.parse::<u64>() {
        return Some(Duration::from_secs(seconds));
    }
    let date = httpdate::parse_http_date(value).ok()?;
    date.duration_since(SystemTime::now()).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_debug_redacts_authorization() {
        let client = Client::builder().api_key("super-secret").build().unwrap();
        let output = format!("{client:?}");
        assert!(!output.contains("super-secret"));
        assert!(output.contains("[REDACTED]"));
    }

    #[test]
    fn builder_debug_redacts_authorization() {
        let builder = Client::builder().api_key("super-secret");
        let output = format!("{builder:?}");
        assert!(!output.contains("super-secret"));
        assert!(output.contains("[REDACTED]"));
    }

    #[test]
    fn insecure_non_loopback_is_rejected() {
        let error = Client::builder()
            .api_key("key")
            .base_url("http://example.com/v1/")
            .allow_insecure_loopback(true)
            .build()
            .unwrap_err();
        assert_eq!(error.kind(), crate::ErrorKind::Configuration);
    }
}
