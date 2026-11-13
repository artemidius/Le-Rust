#![allow(clippy::print_stdout)]

use std::io;

use le_rust::chat::{ChatRequest, FunctionDefinition, Message, Tool, ToolChoice};
use le_rust::{Client, RequestOptions};
use serde_json::{Value, json};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::from_env()?;
    let model = std::env::var("MISTRAL_MODEL")?;
    let tool = Tool::function(
        FunctionDefinition::new(
            "local_time",
            json!({
                "type": "object",
                "properties": { "city": { "type": "string" } },
                "required": ["city"],
                "additionalProperties": false
            }),
        )
        .description("Return the local time for a city")
        .strict(true),
    );

    let first = client
        .chat()
        .complete(
            ChatRequest::new(
                model.clone(),
                vec![Message::user("What time is it in Vienna? Use the tool.")],
            )
            .tools(vec![tool])
            .tool_choice(ToolChoice::Required)
            .parallel_tool_calls(false),
            RequestOptions::default(),
        )
        .await?;

    let assistant = first
        .data
        .choices
        .first()
        .ok_or_else(|| io::Error::other("no choice returned"))?;
    let call = assistant
        .message
        .tool_calls
        .first()
        .ok_or_else(|| io::Error::other("no tool call returned"))?;
    let arguments: Value = call.function.parse_arguments()?;
    println!("tool arguments: {arguments}");

    let call_id = call
        .id
        .clone()
        .ok_or_else(|| io::Error::other("tool call had no ID"))?;
    let messages = vec![
        Message::user("What time is it in Vienna? Use the tool."),
        assistant.message.to_message(),
        Message::tool(
            call_id,
            json!({"time":"12:00","timezone":"Europe/Vienna"}).to_string(),
        ),
    ];
    let final_response = client
        .chat()
        .complete(ChatRequest::new(model, messages), RequestOptions::default())
        .await?;
    if let Some(text) = final_response
        .data
        .choices
        .first()
        .and_then(|choice| choice.message.text())
    {
        println!("{text}");
    }
    Ok(())
}
