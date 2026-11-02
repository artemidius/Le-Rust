#![allow(clippy::print_stdout)]

use le_rust::chat::{ChatRequest, Message};
use le_rust::{Client, RequestOptions};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::from_env()?;
    let model = std::env::var("MISTRAL_MODEL")?;
    let response = client
        .chat()
        .complete(
            ChatRequest::new(
                model,
                vec![
                    Message::system("Answer accurately and concisely."),
                    Message::user("Why does Rust prevent data races?"),
                ],
            ),
            RequestOptions::default(),
        )
        .await?;

    if let Some(text) = response
        .data
        .choices
        .first()
        .and_then(|choice| choice.message.text())
    {
        println!("{text}");
    }
    Ok(())
}
