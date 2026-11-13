#![allow(clippy::print_stdout)]

use le_rust::models::ListModelsOptions;
use le_rust::{Client, RequestOptions};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::from_env()?;
    let models = client.models().list(ListModelsOptions::default()).await?;

    for model in models.data.data {
        println!("{}", model.id);
    }

    if let Ok(model_id) = std::env::var("MISTRAL_MODEL") {
        let model = client
            .models()
            .retrieve(&model_id, RequestOptions::default())
            .await?;
        println!(
            "{} context={:?}",
            model.data.id, model.data.max_context_length
        );
    }
    Ok(())
}
