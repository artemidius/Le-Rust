#![allow(clippy::print_stdout)]

use futures_util::StreamExt;
use le_rust::chat::Content;
use le_rust::fim::FimRequest;
use le_rust::{Client, RequestOptions};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::from_env()?;
    let model = std::env::var("CODESTRAL_MODEL")?;
    let request = FimRequest::new(
        model.clone(),
        "fn add(left: i32, right: i32) -> i32 {\n    ",
    )
    .suffix("\n}\n")
    .max_tokens(64);

    let response = client
        .fim()
        .complete(request, RequestOptions::default())
        .await?;
    if let Some(text) = response
        .data
        .choices
        .first()
        .and_then(|choice| choice.message.text())
    {
        println!("complete: {text}");
    }

    let mut stream = client
        .fim()
        .stream(
            FimRequest::new(model, "fn answer() -> u8 { ").suffix(" }"),
            RequestOptions::default(),
        )
        .await?;
    while let Some(chunk) = stream.next().await {
        for choice in chunk?.choices {
            if let Some(Content::Text(text)) = choice.delta.content {
                print!("{text}");
            }
        }
    }
    println!();
    Ok(())
}
