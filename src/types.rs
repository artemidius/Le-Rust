use std::collections::BTreeMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Response payload together with transport metadata.
#[derive(Debug, Clone)]
pub struct ApiResponse<T> {
    /// Decoded provider payload.
    pub data: T,
    /// Safe response metadata collected from headers.
    pub meta: ResponseMeta,
}

impl<T> ApiResponse<T> {
    pub(crate) fn new(data: T, meta: ResponseMeta) -> Self {
        Self { data, meta }
    }

    /// Transform the payload while preserving response metadata.
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> ApiResponse<U> {
        ApiResponse {
            data: f(self.data),
            meta: self.meta,
        }
    }
}

/// Metadata that is safe to expose without response content.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct ResponseMeta {
    pub request_id: Option<String>,
    pub retry_after: Option<Duration>,
    pub rate_limit_limit: Option<String>,
    pub rate_limit_remaining: Option<String>,
    pub rate_limit_reset: Option<String>,
}

/// Token accounting reported by the provider.
///
/// Counts stay optional because streaming chunks and future API revisions may
/// omit them. Additional fields are preserved without assigning billing meaning.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Usage {
    #[serde(default)]
    pub prompt_tokens: Option<u64>,
    #[serde(default)]
    pub completion_tokens: Option<u64>,
    #[serde(default)]
    pub total_tokens: Option<u64>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}
