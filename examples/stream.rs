#![allow(clippy::print_stdout)]

use std::io::{self, Write};

use futures_util::StreamExt;
use le_rust::chat::{ChatRequest, Content, Message};
use le_rust::{Client, RequestOptions};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::from_env()?;
    let model = std::env::var("MISTRAL_MODEL")?;
    let mut stream = client
        .chat()
        .stream(
            ChatRequest::new(model, vec![Message::user("Write a Rust iterator example.")]),
            RequestOptions::default(),
        )
        .await?;

    while let Some(chunk) = stream.next().await {
        for choice in chunk?.choices {
            if let Some(Content::Text(text)) = choice.delta.content {
                print!("{text}");
                io::stdout().flush()?;
            }
        }
    }
    println!();
    Ok(())
}
