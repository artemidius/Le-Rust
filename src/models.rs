use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

use crate::{ApiResponse, Client, RequestOptions, Result};

#[derive(Debug, Clone)]
pub struct Models<'a> {
    pub(crate) client: &'a Client,
}

#[derive(Debug, Clone, Default)]
/// Filters and per-request transport options for model discovery.
pub struct ListModelsOptions {
    pub provider: Option<String>,
    pub model: Option<String>,
    pub request: RequestOptions,
}

#[derive(Debug, Clone)]
#[non_exhaustive]
/// Envelope returned by the model-list endpoint.
pub struct ModelList {
    pub object: Option<String>,
    pub data: Vec<ModelCard>,
    pub extra: BTreeMap<String, Value>,
}

impl<'de> Deserialize<'de> for ModelList {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum WireModelList {
            Envelope {
                #[serde(default)]
                object: Option<String>,
                #[serde(default)]
                data: Vec<ModelCard>,
                #[serde(flatten)]
                extra: BTreeMap<String, Value>,
            },
            Bare(Vec<ModelCard>),
        }

        Ok(match WireModelList::deserialize(deserializer)? {
            WireModelList::Envelope {
                object,
                data,
                extra,
            } => Self {
                object,
                data,
                extra,
            },
            WireModelList::Bare(data) => Self {
                object: None,
                data,
                extra: BTreeMap::new(),
            },
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
/// A model card with unknown provider fields preserved in `extra`.
pub struct ModelCard {
    pub id: String,
    #[serde(default)]
    pub object: Option<String>,
    #[serde(default)]
    pub created: Option<u64>,
    #[serde(default)]
    pub owned_by: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub max_context_length: Option<u64>,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub capabilities: ModelCapabilities,
    #[serde(default)]
    pub archived: Option<bool>,
    #[serde(default)]
    pub deprecation: Option<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[non_exhaustive]
/// Capability observations reported by the provider.
pub struct ModelCapabilities {
    #[serde(default)]
    pub completion_chat: Option<bool>,
    #[serde(default)]
    pub completion_fim: Option<bool>,
    #[serde(default)]
    pub function_calling: Option<bool>,
    #[serde(default)]
    pub fine_tuning: Option<bool>,
    #[serde(default)]
    pub vision: Option<bool>,
    #[serde(default)]
    pub classification: Option<bool>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl Models<'_> {
    pub async fn list(&self, options: ListModelsOptions) -> Result<ApiResponse<ModelList>> {
        let mut query = Vec::new();
        if let Some(provider) = options.provider {
            query.push(("provider", provider));
        }
        if let Some(model) = options.model {
            query.push(("model", model));
        }
        self.client
            .get_json("models", &query, options.request)
            .await
    }

    pub async fn retrieve(
        &self,
        model_id: &str,
        options: RequestOptions,
    ) -> Result<ApiResponse<ModelCard>> {
        if model_id.is_empty() {
            return Err(crate::Error::InvalidRequest(
                "model ID cannot be empty".into(),
            ));
        }
        let path = self.client.path_with_segment("models", model_id)?;
        self.client.get_json_url(path, &[], options).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_list_accepts_documented_envelope() {
        let list: ModelList = serde_json::from_value(serde_json::json!({
            "object": "list",
            "data": [{"id":"mistral-test"}],
            "future": true
        }))
        .unwrap();
        assert_eq!(list.object.as_deref(), Some("list"));
        assert_eq!(list.data[0].id, "mistral-test");
        assert_eq!(list.extra["future"], true);
    }

    #[test]
    fn model_list_accepts_bare_array_shown_by_api_example() {
        let list: ModelList = serde_json::from_value(serde_json::json!([
            {"id":"mistral-test", "future": 7}
        ]))
        .unwrap();
        assert!(list.object.is_none());
        assert_eq!(list.data[0].id, "mistral-test");
        assert_eq!(list.data[0].extra["future"], 7);
    }
}
