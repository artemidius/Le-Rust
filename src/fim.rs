use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::Value;

use crate::chat::{ChatCompletionResponse, Stop};
use crate::{ApiResponse, Client, CompletionStream, RequestOptions, Result};

#[derive(Debug, Clone)]
pub struct Fim<'a> {
    pub(crate) client: &'a Client,
}

impl Fim<'_> {
    pub async fn complete(
        &self,
        mut request: FimRequest,
        options: RequestOptions,
    ) -> Result<ApiResponse<ChatCompletionResponse>> {
        request.stream = false;
        request.validate()?;
        self.client
            .post_json("fim/completions", &request, options)
            .await
    }

    pub async fn stream(
        &self,
        mut request: FimRequest,
        options: RequestOptions,
    ) -> Result<CompletionStream> {
        request.stream = true;
        request.validate()?;
        self.client
            .post_stream("fim/completions", &request, options)
            .await
    }
}

#[derive(Debug, Clone, Serialize)]
/// Native fill-in-the-middle request with an explicit model and prefix.
pub struct FimRequest {
    model: String,
    prompt: String,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    suffix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    min_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    random_seed: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop: Option<Stop>,
    #[serde(skip_serializing_if = "Option::is_none")]
    prompt_cache_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata: Option<BTreeMap<String, Value>>,
}

impl FimRequest {
    pub fn new(model: impl Into<String>, prompt: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            prompt: prompt.into(),
            stream: false,
            suffix: None,
            max_tokens: None,
            min_tokens: None,
            temperature: None,
            top_p: None,
            random_seed: None,
            stop: None,
            prompt_cache_key: None,
            metadata: None,
        }
    }

    pub fn suffix(mut self, suffix: impl Into<String>) -> Self {
        self.suffix = Some(suffix.into());
        self
    }

    pub fn max_tokens(mut self, max_tokens: u64) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    pub fn min_tokens(mut self, min_tokens: u64) -> Self {
        self.min_tokens = Some(min_tokens);
        self
    }

    pub fn temperature(mut self, temperature: f64) -> Self {
        self.temperature = Some(temperature);
        self
    }

    pub fn top_p(mut self, top_p: f64) -> Self {
        self.top_p = Some(top_p);
        self
    }

    pub fn random_seed(mut self, seed: i64) -> Self {
        self.random_seed = Some(seed);
        self
    }

    pub fn stop(mut self, stop: impl Into<Stop>) -> Self {
        self.stop = Some(stop.into());
        self
    }

    pub fn prompt_cache_key(mut self, key: impl Into<String>) -> Self {
        self.prompt_cache_key = Some(key.into());
        self
    }

    pub fn metadata(mut self, metadata: BTreeMap<String, Value>) -> Self {
        self.metadata = Some(metadata);
        self
    }

    fn validate(&self) -> Result<()> {
        if self.model.trim().is_empty() {
            return Err(crate::Error::InvalidRequest("model cannot be empty".into()));
        }
        for (name, value) in [("temperature", self.temperature), ("top_p", self.top_p)] {
            if value.is_some_and(|value| !value.is_finite()) {
                return Err(crate::Error::InvalidRequest(format!(
                    "{name} must be finite"
                )));
            }
        }
        Ok(())
    }
}
