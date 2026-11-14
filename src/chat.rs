use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

use crate::{ApiResponse, Client, CompletionStream, RequestOptions, Result, Usage};

#[derive(Debug, Clone)]
pub struct Chat<'a> {
    pub(crate) client: &'a Client,
}

impl Chat<'_> {
    pub async fn complete(
        &self,
        mut request: ChatRequest,
        options: RequestOptions,
    ) -> Result<ApiResponse<ChatCompletionResponse>> {
        request.stream = false;
        request.validate()?;
        self.client
            .post_json("chat/completions", &request, options)
            .await
    }

    pub async fn stream(
        &self,
        mut request: ChatRequest,
        options: RequestOptions,
    ) -> Result<CompletionStream> {
        request.stream = true;
        request.validate()?;
        self.client
            .post_stream("chat/completions", &request, options)
            .await
    }
}

/// A native chat-completion request with an explicit model and message history.
#[derive(Debug, Clone, Serialize)]
pub struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<Tool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<ToolChoice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    parallel_tool_calls: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<ResponseFormat>,
    #[serde(skip_serializing_if = "Option::is_none")]
    prediction: Option<Prediction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    min_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    random_seed: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    presence_penalty: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    frequency_penalty: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    repetition_penalty: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_k: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    n: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    logprobs: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_logprobs: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    prompt_logprobs: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_prompt_logprobs: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop: Option<Stop>,
    #[serde(skip_serializing_if = "Option::is_none")]
    safe_prompt: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    prompt_cache_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reasoning_effort: Option<ReasoningEffort>,
    #[serde(skip_serializing_if = "Option::is_none")]
    prompt_mode: Option<PromptMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    service_tier: Option<ServiceTier>,
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata: Option<BTreeMap<String, Value>>,
    #[serde(flatten)]
    extra: BTreeMap<String, Value>,
}

impl ChatRequest {
    pub fn new(model: impl Into<String>, messages: Vec<Message>) -> Self {
        Self {
            model: model.into(),
            messages,
            stream: false,
            tools: None,
            tool_choice: None,
            parallel_tool_calls: None,
            response_format: None,
            prediction: None,
            temperature: None,
            top_p: None,
            max_tokens: None,
            min_tokens: None,
            random_seed: None,
            presence_penalty: None,
            frequency_penalty: None,
            repetition_penalty: None,
            top_k: None,
            n: None,
            logprobs: None,
            top_logprobs: None,
            prompt_logprobs: None,
            top_prompt_logprobs: None,
            stop: None,
            safe_prompt: None,
            prompt_cache_key: None,
            reasoning_effort: None,
            prompt_mode: None,
            service_tier: None,
            metadata: None,
            extra: BTreeMap::new(),
        }
    }

    pub fn tools(mut self, tools: Vec<Tool>) -> Self {
        self.tools = Some(tools);
        self
    }

    pub fn tool_choice(mut self, choice: ToolChoice) -> Self {
        self.tool_choice = Some(choice);
        self
    }

    pub fn parallel_tool_calls(mut self, enabled: bool) -> Self {
        self.parallel_tool_calls = Some(enabled);
        self
    }

    pub fn response_format(mut self, format: ResponseFormat) -> Self {
        self.response_format = Some(format);
        self
    }

    pub fn prediction(mut self, prediction: Prediction) -> Self {
        self.prediction = Some(prediction);
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

    pub fn max_tokens(mut self, max_tokens: u64) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    pub fn min_tokens(mut self, min_tokens: u64) -> Self {
        self.min_tokens = Some(min_tokens);
        self
    }

    pub fn random_seed(mut self, seed: i64) -> Self {
        self.random_seed = Some(seed);
        self
    }

    pub fn presence_penalty(mut self, penalty: f64) -> Self {
        self.presence_penalty = Some(penalty);
        self
    }

    pub fn frequency_penalty(mut self, penalty: f64) -> Self {
        self.frequency_penalty = Some(penalty);
        self
    }

    pub fn repetition_penalty(mut self, penalty: f64) -> Self {
        self.repetition_penalty = Some(penalty);
        self
    }

    pub fn top_k(mut self, top_k: i64) -> Self {
        self.top_k = Some(top_k);
        self
    }

    pub fn choices(mut self, choices: u64) -> Self {
        self.n = Some(choices);
        self
    }

    pub fn logprobs(mut self, enabled: bool) -> Self {
        self.logprobs = Some(enabled);
        self
    }

    pub fn top_logprobs(mut self, count: u64) -> Self {
        self.top_logprobs = Some(count);
        self
    }

    pub fn prompt_logprobs(mut self, enabled: bool) -> Self {
        self.prompt_logprobs = Some(enabled);
        self
    }

    pub fn top_prompt_logprobs(mut self, count: u64) -> Self {
        self.top_prompt_logprobs = Some(count);
        self
    }

    pub fn stop(mut self, stop: impl Into<Stop>) -> Self {
        self.stop = Some(stop.into());
        self
    }

    pub fn safe_prompt(mut self, enabled: bool) -> Self {
        self.safe_prompt = Some(enabled);
        self
    }

    pub fn prompt_cache_key(mut self, key: impl Into<String>) -> Self {
        self.prompt_cache_key = Some(key.into());
        self
    }

    pub fn reasoning_effort(mut self, effort: ReasoningEffort) -> Self {
        self.reasoning_effort = Some(effort);
        self
    }

    pub fn prompt_mode(mut self, mode: PromptMode) -> Self {
        self.prompt_mode = Some(mode);
        self
    }

    pub fn service_tier(mut self, tier: ServiceTier) -> Self {
        self.service_tier = Some(tier);
        self
    }

    pub fn metadata(mut self, metadata: BTreeMap<String, Value>) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Add a native request field not yet represented by the typed API.
    ///
    /// Reserved fields are rejected during validation and can never override a
    /// typed setting or transport-controlled stream mode.
    pub fn extension(mut self, key: impl Into<String>, value: Value) -> Self {
        self.extra.insert(key.into(), value);
        self
    }

    fn validate(&self) -> Result<()> {
        if self.model.trim().is_empty() {
            return Err(crate::Error::InvalidRequest("model cannot be empty".into()));
        }
        if self.messages.is_empty() {
            return Err(crate::Error::InvalidRequest(
                "at least one chat message is required".into(),
            ));
        }
        for key in self.extra.keys() {
            if RESERVED_CHAT_FIELDS.contains(&key.as_str()) {
                return Err(crate::Error::InvalidRequest(format!(
                    "extension field `{key}` collides with a native field"
                )));
            }
        }
        validate_inclusive("temperature", self.temperature, 0.0, 1.5)?;
        validate_exclusive_inclusive("top_p", self.top_p, 0.0, 1.0)?;
        validate_inclusive("presence_penalty", self.presence_penalty, -2.0, 2.0)?;
        validate_inclusive("frequency_penalty", self.frequency_penalty, -2.0, 2.0)?;
        validate_exclusive_inclusive("repetition_penalty", self.repetition_penalty, 0.0, 2.0)?;
        if self.random_seed.is_some_and(|value| value < 0) {
            return Err(crate::Error::InvalidRequest(
                "random_seed must be zero or greater".into(),
            ));
        }
        if self.top_k.is_some_and(|value| value < -1) {
            return Err(crate::Error::InvalidRequest(
                "top_k must be -1 or greater".into(),
            ));
        }
        if self.n == Some(0) {
            return Err(crate::Error::InvalidRequest(
                "number of choices must be at least one".into(),
            ));
        }
        Ok(())
    }
}

fn validate_inclusive(name: &str, value: Option<f64>, minimum: f64, maximum: f64) -> Result<()> {
    if value.is_some_and(|value| !value.is_finite() || value < minimum || value > maximum) {
        return Err(crate::Error::InvalidRequest(format!(
            "{name} must be between {minimum} and {maximum}, inclusive"
        )));
    }
    Ok(())
}

fn validate_exclusive_inclusive(
    name: &str,
    value: Option<f64>,
    minimum: f64,
    maximum: f64,
) -> Result<()> {
    if value.is_some_and(|value| !value.is_finite() || value <= minimum || value > maximum) {
        return Err(crate::Error::InvalidRequest(format!(
            "{name} must be greater than {minimum} and no greater than {maximum}"
        )));
    }
    Ok(())
}

const RESERVED_CHAT_FIELDS: &[&str] = &[
    "model",
    "messages",
    "stream",
    "tools",
    "tool_choice",
    "parallel_tool_calls",
    "response_format",
    "prediction",
    "temperature",
    "top_p",
    "max_tokens",
    "min_tokens",
    "random_seed",
    "presence_penalty",
    "frequency_penalty",
    "repetition_penalty",
    "top_k",
    "n",
    "logprobs",
    "top_logprobs",
    "prompt_logprobs",
    "top_prompt_logprobs",
    "stop",
    "safe_prompt",
    "prompt_cache_key",
    "reasoning_effort",
    "prompt_mode",
    "service_tier",
    "metadata",
];

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "role", rename_all = "lowercase")]
/// Role-specific request message.
pub enum Message {
    System {
        content: Content,
    },
    User {
        content: Content,
    },
    Assistant {
        #[serde(skip_serializing_if = "Option::is_none")]
        content: Option<Content>,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        tool_calls: Vec<ToolCall>,
        #[serde(skip_serializing_if = "Option::is_none")]
        prefix: Option<bool>,
        #[serde(flatten)]
        extra: BTreeMap<String, Value>,
    },
    Tool {
        content: Content,
        tool_call_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
}

impl Message {
    pub fn system(content: impl Into<Content>) -> Self {
        Self::System {
            content: content.into(),
        }
    }

    pub fn user(content: impl Into<Content>) -> Self {
        Self::User {
            content: content.into(),
        }
    }

    pub fn assistant(content: impl Into<Content>) -> Self {
        Self::Assistant {
            content: Some(content.into()),
            tool_calls: Vec::new(),
            prefix: None,
            extra: BTreeMap::new(),
        }
    }

    pub fn assistant_tool_calls(tool_calls: Vec<ToolCall>) -> Self {
        Self::Assistant {
            content: None,
            tool_calls,
            prefix: None,
            extra: BTreeMap::new(),
        }
    }

    pub fn tool(tool_call_id: impl Into<String>, content: impl Into<Content>) -> Self {
        Self::Tool {
            content: content.into(),
            tool_call_id: tool_call_id.into(),
            name: None,
        }
    }

    /// Construct a tool-result message that retains the called function's name.
    ///
    /// Mistral's documented function-calling loop includes both `name` and
    /// `tool_call_id` when returning a function result to the model.
    pub fn named_tool(
        tool_call_id: impl Into<String>,
        name: impl Into<String>,
        content: impl Into<Content>,
    ) -> Self {
        Self::Tool {
            content: content.into(),
            tool_call_id: tool_call_id.into(),
            name: Some(name.into()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
/// Text or losslessly preserved native content blocks.
pub enum Content {
    Text(String),
    Blocks(Vec<Value>),
}

impl From<String> for Content {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}

impl From<&str> for Content {
    fn from(value: &str) -> Self {
        Self::Text(value.to_owned())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Tool {
    #[serde(rename = "type")]
    kind: &'static str,
    pub function: FunctionDefinition,
}

impl Tool {
    pub fn function(function: FunctionDefinition) -> Self {
        Self {
            kind: "function",
            function,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
/// Function-tool declaration.
pub struct FunctionDefinition {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub parameters: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
}

impl FunctionDefinition {
    pub fn new(name: impl Into<String>, parameters: Value) -> Self {
        Self {
            name: name.into(),
            description: None,
            parameters,
            strict: None,
        }
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn strict(mut self, strict: bool) -> Self {
        self.strict = Some(strict);
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
/// Native tool-selection policy or one named function.
pub enum ToolChoice {
    Auto,
    None,
    Any,
    Required,
    Function(String),
}

impl Serialize for ToolChoice {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Auto => serializer.serialize_str("auto"),
            Self::None => serializer.serialize_str("none"),
            Self::Any => serializer.serialize_str("any"),
            Self::Required => serializer.serialize_str("required"),
            Self::Function(name) => serde_json::json!({
                "type": "function",
                "function": { "name": name }
            })
            .serialize(serializer),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
/// Native response format requested from a chat model.
pub enum ResponseFormat {
    Text,
    JsonObject,
    JsonSchema { json_schema: JsonSchemaDefinition },
}

#[derive(Debug, Clone, Serialize)]
pub struct JsonSchemaDefinition {
    pub name: String,
    #[serde(rename = "schema")]
    pub schema_value: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
}

impl JsonSchemaDefinition {
    pub fn new(name: impl Into<String>, schema: Value) -> Self {
        Self {
            name: name.into(),
            schema_value: schema,
            description: None,
            strict: None,
        }
    }

    pub fn strict(mut self, strict: bool) -> Self {
        self.strict = Some(strict);
        self
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Prediction {
    Content { content: String },
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum Stop {
    One(String),
    Many(Vec<String>),
}

impl From<String> for Stop {
    fn from(value: String) -> Self {
        Self::One(value)
    }
}

impl From<&str> for Stop {
    fn from(value: &str) -> Self {
        Self::One(value.to_owned())
    }
}

impl From<Vec<String>> for Stop {
    fn from(value: Vec<String>) -> Self {
        Self::Many(value)
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ReasoningEffort {
    None,
    Minimal,
    Low,
    Medium,
    High,
    Xhigh,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PromptMode {
    Reasoning,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceTier {
    Auto,
    StandardOnly,
}

#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
/// Complete response shared by chat and FIM completions.
pub struct ChatCompletionResponse {
    pub id: String,
    pub model: String,
    #[serde(default)]
    pub object: Option<String>,
    #[serde(default)]
    pub created: Option<u64>,
    #[serde(default)]
    pub choices: Vec<ChatChoice>,
    #[serde(default)]
    pub usage: Option<Usage>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl ChatCompletionResponse {
    /// Return text from the first standard completion choice.
    ///
    /// This convenience accessor is intentionally fallible: an empty choice
    /// list, an alternate multi-message response, and structured block content
    /// are distinct provider responses and are not converted into empty text.
    pub fn first_text(&self) -> std::result::Result<&str, CompletionTextError> {
        let choice = self
            .choices
            .first()
            .ok_or(CompletionTextError::MissingChoice)?;
        let message = choice
            .message()
            .ok_or(CompletionTextError::MissingMessage)?;
        message.text().ok_or(CompletionTextError::NonTextContent)
    }
}

#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct ChatChoice {
    pub index: usize,
    #[serde(default)]
    pub message: Option<AssistantResponseMessage>,
    /// Alternate native multi-completion payload used by some tool families.
    #[serde(default)]
    pub messages: Vec<DeltaMessage>,
    #[serde(default)]
    pub finish_reason: Option<FinishReason>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl ChatChoice {
    /// Return the ordinary assistant message when this is a standard completion.
    pub fn message(&self) -> Option<&AssistantResponseMessage> {
        self.message.as_ref()
    }
}

#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct AssistantResponseMessage {
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub content: Option<Content>,
    #[serde(default)]
    pub tool_calls: Vec<ToolCall>,
    #[serde(default)]
    pub prefix: Option<bool>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl AssistantResponseMessage {
    pub fn text(&self) -> Option<&str> {
        match self.content.as_ref()? {
            Content::Text(text) => Some(text),
            Content::Blocks(_) => None,
        }
    }

    /// Deserialize a text response into an application type.
    ///
    /// This verifies JSON syntax and the requested Rust shape. It does not
    /// replace application-level validation.
    pub fn parse_json<T: serde::de::DeserializeOwned>(
        &self,
    ) -> std::result::Result<T, OutputParseError> {
        let text = self.text().ok_or(OutputParseError::MissingText)?;
        serde_json::from_str(text).map_err(OutputParseError::Json)
    }

    /// Convert a response to a replayable assistant message without flattening content.
    pub fn to_message(&self) -> Message {
        Message::Assistant {
            content: self.content.clone(),
            tool_calls: self.tool_calls.clone(),
            prefix: self.prefix,
            extra: self.extra.clone(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum OutputParseError {
    #[error("assistant response did not contain text")]
    MissingText,
    #[error("assistant response text was not valid for the requested JSON type")]
    Json(#[source] serde_json::Error),
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum CompletionTextError {
    #[error("completion response did not contain a choice")]
    MissingChoice,
    #[error("first completion choice did not contain a standard assistant message")]
    MissingMessage,
    #[error("first completion message did not contain plain text")]
    NonTextContent,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ToolCall {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, rename = "type", skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    pub function: FunctionCall,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index: Option<usize>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl ToolCall {
    /// Construct a native function tool call for assistant-history replay.
    pub fn function(
        id: impl Into<String>,
        name: impl Into<String>,
        arguments: impl Into<ToolArguments>,
    ) -> Self {
        Self {
            id: Some(id.into()),
            kind: Some("function".into()),
            function: FunctionCall::new(name, arguments),
            index: None,
            extra: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct FunctionCall {
    pub name: String,
    pub arguments: ToolArguments,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl FunctionCall {
    /// Construct a function call for a native tool-call message.
    pub fn new(name: impl Into<String>, arguments: impl Into<ToolArguments>) -> Self {
        Self {
            name: name.into(),
            arguments: arguments.into(),
            extra: BTreeMap::new(),
        }
    }

    pub fn parse_arguments<T: serde::de::DeserializeOwned>(&self) -> serde_json::Result<T> {
        match &self.arguments {
            ToolArguments::Json(value) => serde_json::from_value(value.clone()),
            ToolArguments::String(value) => serde_json::from_str(value),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolArguments {
    String(String),
    Json(Value),
}

impl From<String> for ToolArguments {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<&str> for ToolArguments {
    fn from(value: &str) -> Self {
        Self::String(value.to_owned())
    }
}

impl From<Value> for ToolArguments {
    fn from(value: Value) -> Self {
        Self::Json(value)
    }
}

#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
/// One native completion-stream payload.
pub struct CompletionChunk {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub object: Option<String>,
    #[serde(default)]
    pub created: Option<u64>,
    #[serde(default)]
    pub choices: Vec<ChunkChoice>,
    #[serde(default)]
    pub usage: Option<Usage>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct ChunkChoice {
    pub index: usize,
    #[serde(default)]
    pub delta: DeltaMessage,
    #[serde(default)]
    pub finish_reason: Option<FinishReason>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[non_exhaustive]
pub struct DeltaMessage {
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub content: Option<Content>,
    #[serde(default)]
    pub tool_calls: Vec<ToolCallDelta>,
    #[serde(default)]
    pub tool_call_id: Option<String>,
    #[serde(default)]
    pub index: Option<usize>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct ToolCallDelta {
    #[serde(default)]
    pub index: Option<usize>,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default, rename = "type")]
    pub kind: Option<String>,
    #[serde(default)]
    pub function: Option<FunctionCallDelta>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct FunctionCallDelta {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub arguments: Option<ToolArguments>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum FinishReason {
    Stop,
    Length,
    ModelLength,
    ToolCalls,
    Error,
    Unknown(String),
}

impl FinishReason {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Stop => "stop",
            Self::Length => "length",
            Self::ModelLength => "model_length",
            Self::ToolCalls => "tool_calls",
            Self::Error => "error",
            Self::Unknown(value) => value,
        }
    }
}

impl Serialize for FinishReason {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for FinishReason {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Ok(match value.as_str() {
            "stop" => Self::Stop,
            "length" => Self::Length,
            "model_length" => Self::ModelLength,
            "tool_calls" => Self::ToolCalls,
            "error" => Self::Error,
            _ => Self::Unknown(value),
        })
    }
}

impl fmt::Display for FinishReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_finish_reason_is_preserved() {
        let reason: FinishReason = serde_json::from_str("\"future_reason\"").unwrap();
        assert_eq!(reason, FinishReason::Unknown("future_reason".into()));
    }

    #[test]
    fn schema_uses_native_schema_key() {
        let format = ResponseFormat::JsonSchema {
            json_schema: JsonSchemaDefinition::new("answer", serde_json::json!({"type":"object"})),
        };
        let value = serde_json::to_value(format).unwrap();
        assert!(value["json_schema"].get("schema").is_some());
        assert!(value["json_schema"].get("schema_value").is_none());
    }

    #[test]
    fn reserved_extension_is_rejected() {
        let request = ChatRequest::new("model", vec![Message::user("hello")])
            .extension("stream", Value::Bool(false));
        assert!(request.validate().is_err());
    }

    #[test]
    fn assistant_replay_preserves_unknown_fields() {
        let response: AssistantResponseMessage = serde_json::from_value(serde_json::json!({
            "role": "assistant",
            "content": "answer",
            "reasoning_signature": "opaque"
        }))
        .unwrap();
        let replay = serde_json::to_value(response.to_message()).unwrap();
        assert_eq!(replay["reasoning_signature"], "opaque");
    }

    #[test]
    fn sampling_and_observability_options_use_native_keys() {
        let request = ChatRequest::new("model", vec![Message::user("hello")])
            .min_tokens(2)
            .presence_penalty(0.1)
            .frequency_penalty(0.2)
            .repetition_penalty(1.1)
            .top_k(8)
            .choices(2)
            .logprobs(true)
            .top_logprobs(3)
            .prompt_logprobs(true)
            .top_prompt_logprobs(4)
            .prompt_mode(PromptMode::Reasoning)
            .service_tier(ServiceTier::StandardOnly);
        let value = serde_json::to_value(request).unwrap();
        assert_eq!(value["min_tokens"], 2);
        assert_eq!(value["n"], 2);
        assert_eq!(value["prompt_mode"], "reasoning");
        assert_eq!(value["service_tier"], "standard_only");
        assert_eq!(value["top_prompt_logprobs"], 4);
    }

    #[test]
    fn documented_numeric_ranges_are_validated() {
        let base = || ChatRequest::new("model", vec![Message::user("hello")]);
        assert!(base().temperature(1.5).validate().is_ok());
        assert!(base().temperature(1.500_001).validate().is_err());
        assert!(base().top_p(0.0).validate().is_err());
        assert!(base().presence_penalty(-2.0).validate().is_ok());
        assert!(base().frequency_penalty(2.1).validate().is_err());
        assert!(base().repetition_penalty(0.0).validate().is_err());
        assert!(base().random_seed(-1).validate().is_err());
        assert!(base().top_k(-1).validate().is_ok());
        assert!(base().top_k(-2).validate().is_err());
        assert!(base().choices(0).validate().is_err());
    }

    #[test]
    fn alternate_multi_completion_shape_is_preserved() {
        let choice: ChatChoice = serde_json::from_value(serde_json::json!({
            "index": 0,
            "messages": [{"role":"assistant", "content":"server tool output"}],
            "finish_reason": "stop"
        }))
        .unwrap();
        assert!(choice.message.is_none());
        assert_eq!(choice.messages.len(), 1);
        assert_eq!(
            choice.messages[0].content,
            Some(Content::Text("server tool output".into()))
        );
    }

    #[test]
    fn structured_output_parser_is_explicit_and_typed() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct Answer {
            value: u8,
        }

        let message: AssistantResponseMessage = serde_json::from_value(serde_json::json!({
            "role": "assistant",
            "content": "{\"value\":7}"
        }))
        .unwrap();
        assert_eq!(message.parse_json::<Answer>().unwrap(), Answer { value: 7 });

        let blocks: AssistantResponseMessage = serde_json::from_value(serde_json::json!({
            "role": "assistant",
            "content": [{"type":"text", "text":"{}"}]
        }))
        .unwrap();
        assert!(matches!(
            blocks.parse_json::<Answer>(),
            Err(OutputParseError::MissingText)
        ));
    }

    #[test]
    fn completion_text_accessor_is_fallible() {
        let response: ChatCompletionResponse = serde_json::from_value(serde_json::json!({
            "id": "cmpl_1",
            "model": "model",
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": "middle"},
                "finish_reason": "stop"
            }]
        }))
        .unwrap();
        assert_eq!(response.first_text().unwrap(), "middle");

        let empty: ChatCompletionResponse = serde_json::from_value(serde_json::json!({
            "id": "cmpl_2",
            "model": "model",
            "choices": []
        }))
        .unwrap();
        assert!(matches!(
            empty.first_text(),
            Err(CompletionTextError::MissingChoice)
        ));
    }

    #[test]
    fn tool_call_constructors_produce_replayable_native_message() {
        let call = ToolCall::function("call-1", "read_file", "{\"path\":\"src/lib.rs\"}");
        let value = serde_json::to_value(Message::assistant_tool_calls(vec![call])).unwrap();
        assert_eq!(value["role"], "assistant");
        assert_eq!(value["tool_calls"][0]["id"], "call-1");
        assert_eq!(value["tool_calls"][0]["type"], "function");
        assert_eq!(value["tool_calls"][0]["function"]["name"], "read_file");
        assert!(value["tool_calls"][0].get("index").is_none());
    }

    #[test]
    fn absent_optional_tool_call_fields_are_omitted_instead_of_null() {
        let call: ToolCall = serde_json::from_value(serde_json::json!({
            "function": {"name": "read_file", "arguments": "{}"}
        }))
        .unwrap();

        let value = serde_json::to_value(call).unwrap();
        assert!(value.get("id").is_none());
        assert!(value.get("type").is_none());
        assert!(value.get("index").is_none());
    }

    #[test]
    fn named_tool_constructor_preserves_function_identity() {
        let value =
            serde_json::to_value(Message::named_tool("call-1", "read_file", "file contents"))
                .unwrap();
        assert_eq!(value["role"], "tool");
        assert_eq!(value["tool_call_id"], "call-1");
        assert_eq!(value["name"], "read_file");
        assert_eq!(value["content"], "file contents");
    }
}
