use std::convert::Infallible;
use std::io;
use std::sync::Arc;

use axum::Router;
use axum::body::{Body, Bytes, to_bytes};
use axum::extract::State;
use axum::http::{Request, Response, StatusCode, header};
use futures_util::stream;
use le_rust::chat::{
    ChatRequest, FunctionDefinition, JsonSchemaDefinition, Message, ResponseFormat, Tool,
    ToolChoice,
};
use le_rust::fim::FimRequest;
use le_rust::models::ListModelsOptions;
use le_rust::{Client, RequestOptions};
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::net::TcpListener;
use tokio::sync::Mutex;

#[derive(Default)]
struct StateData {
    paths: Mutex<Vec<String>>,
}

async fn handler(State(state): State<Arc<StateData>>, request: Request<Body>) -> Response<Body> {
    let path = request.uri().path().to_owned();
    let bytes = to_bytes(request.into_body(), 1024 * 1024).await.unwrap();
    let body: Value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    };
    state.paths.lock().await.push(path.clone());

    match path.as_str() {
        "/v1/models" => json_response(json!({
            "object": "list",
            "data": [{
                "id": "mistral-consumer-test",
                "capabilities": {
                    "completion_chat": true,
                    "completion_fim": true,
                    "function_calling": true
                }
            }]
        })),
        "/v1/models/mistral-consumer-test" => json_response(json!({
            "id": "mistral-consumer-test",
            "capabilities": {"completion_chat": true}
        })),
        "/v1/chat/completions" if body["stream"] == true => stream_response(vec![
            "data: {\"id\":\"chat-consumer\",\"model\":\"mistral-consumer-test\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\",\"tool_calls\":[{\"index\":0,\"id\":\"call-time\",\"type\":\"function\",\"function\":{\"name\":\"local_time\",\"arguments\":\"{\\\"city\\\":\"}}]}}]}\n\n",
            "data: {\"id\":\"chat-consumer\",\"model\":\"mistral-consumer-test\",\"choices\":[{\"index\":0,\"delta\":{\"tool_calls\":[{\"index\":0,\"function\":{\"arguments\":\"\\\"Vienna\\\"}\"}}]},\"finish_reason\":\"tool_calls\"}]}\n\n",
            "data: [DONE]\n\n",
        ]),
        "/v1/chat/completions" => json_response(json!({
            "id": "chat-final",
            "model": "mistral-consumer-test",
            "choices": [{
                "index": 0,
                "message": {"role":"assistant", "content":"{\"answer\":\"12:00\"}"},
                "finish_reason": "stop"
            }],
            "usage": {"prompt_tokens": 12, "completion_tokens": 4, "total_tokens": 16}
        })),
        "/v1/fim/completions" if body["stream"] == true => stream_response(vec![
            "data: {\"id\":\"fim-stream\",\"model\":\"codestral-consumer-test\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"42\"},\"finish_reason\":\"stop\"}]}\n\n",
            "data: [DONE]\n\n",
        ]),
        "/v1/fim/completions" => json_response(json!({
            "id": "fim-complete",
            "model": "codestral-consumer-test",
            "choices": [{
                "index": 0,
                "message": {"role":"assistant", "content":"left + right"},
                "finish_reason": "stop"
            }]
        })),
        _ => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::empty())
            .unwrap(),
    }
}

fn json_response(value: Value) -> Response<Body> {
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(value.to_string()))
        .unwrap()
}

fn stream_response(parts: Vec<&'static str>) -> Response<Body> {
    let body = stream::iter(
        parts
            .into_iter()
            .map(|part| Ok::<Bytes, Infallible>(Bytes::from_static(part.as_bytes()))),
    );
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/event-stream")
        .body(Body::from_stream(body))
        .unwrap()
}

#[derive(Debug, Deserialize, PartialEq)]
struct StructuredAnswer {
    answer: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let state = Arc::new(StateData::default());
    let app = Router::new().fallback(handler).with_state(state.clone());
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    let client = Client::builder()
        .api_key("package-consumer-secret")
        .base_url(format!("http://{address}/v1/"))
        .allow_insecure_loopback(true)
        .build()?;

    let models = client.models().list(ListModelsOptions::default()).await?;
    assert_eq!(models.data.data[0].id, "mistral-consumer-test");
    let model = client
        .models()
        .retrieve("mistral-consumer-test", RequestOptions::default())
        .await?;
    assert_eq!(model.data.id, "mistral-consumer-test");

    let tool = Tool::function(FunctionDefinition::new(
        "local_time",
        json!({
            "type": "object",
            "properties": {"city": {"type": "string"}},
            "required": ["city"]
        }),
    ));
    let streamed = client
        .chat()
        .stream(
            ChatRequest::new(
                "mistral-consumer-test",
                vec![Message::user("What time is it in Vienna?")],
            )
            .tools(vec![tool])
            .tool_choice(ToolChoice::Required),
            RequestOptions::default(),
        )
        .await?
        .collect_response()
        .await?;
    let assistant = streamed.choices[0]
        .message()
        .ok_or_else(|| io::Error::other("stream did not collect an assistant message"))?;
    let call = &assistant.tool_calls[0];
    let arguments: Value = call.function.parse_arguments()?;
    assert_eq!(arguments, json!({"city":"Vienna"}));

    let final_response = client
        .chat()
        .complete(
            ChatRequest::new(
                "mistral-consumer-test",
                vec![
                    Message::user("What time is it in Vienna?"),
                    assistant.to_message(),
                    Message::tool(
                        call.id
                            .as_deref()
                            .ok_or_else(|| io::Error::other("tool call had no ID"))?,
                        "{\"time\":\"12:00\"}",
                    ),
                ],
            )
            .response_format(ResponseFormat::JsonSchema {
                json_schema: JsonSchemaDefinition::new(
                    "answer",
                    json!({
                        "type": "object",
                        "properties": {"answer": {"type": "string"}},
                        "required": ["answer"]
                    }),
                ),
            }),
            RequestOptions::default(),
        )
        .await?;
    let answer: StructuredAnswer = final_response.data.choices[0]
        .message()
        .ok_or_else(|| io::Error::other("complete response had no assistant message"))?
        .parse_json()?;
    assert_eq!(
        answer,
        StructuredAnswer {
            answer: "12:00".into()
        }
    );

    let fim = client
        .fim()
        .complete(
            FimRequest::new("codestral-consumer-test", "fn add() { ").suffix(" }"),
            RequestOptions::default(),
        )
        .await?;
    assert_eq!(fim.data.first_text()?, "left + right");
    let streamed_fim = client
        .fim()
        .stream(
            FimRequest::new("codestral-consumer-test", "const VALUE: u8 = "),
            RequestOptions::default(),
        )
        .await?
        .collect_response()
        .await?;
    assert_eq!(streamed_fim.first_text()?, "42");

    assert_eq!(state.paths.lock().await.len(), 6);
    Ok(())
}
