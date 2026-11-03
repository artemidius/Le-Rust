#![cfg(feature = "tracing")]

use std::io;
use std::sync::{Arc, Mutex};

use axum::Router;
use axum::body::Body;
use axum::http::{Response, StatusCode, header};
use le_rust::chat::{ChatRequest, Message};
use le_rust::{Client, RequestOptions};
use tokio::net::TcpListener;
use tracing_subscriber::fmt::MakeWriter;

#[derive(Clone)]
struct CapturedTrace(Arc<Mutex<Vec<u8>>>);

struct CapturedTraceWriter(Arc<Mutex<Vec<u8>>>);

impl<'writer> MakeWriter<'writer> for CapturedTrace {
    type Writer = CapturedTraceWriter;

    fn make_writer(&'writer self) -> Self::Writer {
        CapturedTraceWriter(self.0.clone())
    }
}

impl io::Write for CapturedTraceWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

async fn completion() -> Response<Body> {
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            r#"{"id":"chat-1","model":"mistral-test","choices":[{"index":0,"message":{"role":"assistant","content":"response-secret"}}]}"#,
        ))
        .unwrap()
}

#[tokio::test(flavor = "current_thread")]
async fn tracing_contains_metadata_but_not_credentials_or_content() {
    let output = Arc::new(Mutex::new(Vec::new()));
    let subscriber = tracing_subscriber::fmt()
        .without_time()
        .with_ansi(false)
        .with_max_level(tracing::Level::DEBUG)
        .with_writer(CapturedTrace(output.clone()))
        .finish();
    let _default = tracing::subscriber::set_default(subscriber);

    let app = Router::new().fallback(completion);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let client = Client::builder()
        .api_key("trace-api-secret")
        .base_url(format!("http://{address}/v1/"))
        .allow_insecure_loopback(true)
        .build()
        .unwrap();
    client
        .chat()
        .complete(
            ChatRequest::new("mistral-test", vec![Message::user("trace-prompt-secret")]),
            RequestOptions::default(),
        )
        .await
        .unwrap();

    let output = String::from_utf8(output.lock().unwrap().clone()).unwrap();
    assert!(output.contains("method=\"POST\""), "{output:?}");
    assert!(
        output.contains("endpoint=\"/v1/chat/completions\""),
        "{output:?}"
    );
    assert!(output.contains("status=200"), "{output:?}");
    assert!(!output.contains("trace-api-secret"));
    assert!(!output.contains("trace-prompt-secret"));
    assert!(!output.contains("response-secret"));
}
