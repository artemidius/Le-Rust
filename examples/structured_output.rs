#![allow(clippy::print_stdout)]

use std::io;

use le_rust::chat::{ChatRequest, JsonSchemaDefinition, Message, ResponseFormat};
use le_rust::{Client, RequestOptions};
use serde::Deserialize;
use serde_json::json;

#[derive(Debug, Deserialize)]
struct Language {
    name: String,
    year: u16,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::from_env()?;
    let model = std::env::var("MISTRAL_MODEL")?;
    let schema = JsonSchemaDefinition::new(
        "language",
        json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "year": { "type": "integer" }
            },
            "required": ["name", "year"],
            "additionalProperties": false
        }),
    )
    .strict(true);

    let response = client
        .chat()
        .complete(
            ChatRequest::new(
                model,
                vec![Message::user(
                    "Return JSON with the name Rust and its first stable release year.",
                )],
            )
            .response_format(ResponseFormat::JsonSchema {
                json_schema: schema,
            }),
            RequestOptions::default(),
        )
        .await?;
    let text = response
        .data
        .choices
        .first()
        .and_then(|choice| choice.message.text())
        .ok_or_else(|| io::Error::other("response did not contain text JSON"))?;
    let language: Language = serde_json::from_str(text)?;
    println!("{} {}", language.name, language.year);
    Ok(())
}
