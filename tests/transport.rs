use std::convert::Infallible;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use axum::Router;
use axum::body::{Body, Bytes, to_bytes};
use axum::extract::State;
use axum::http::{Request, Response, StatusCode, header};
use futures_util::{StreamExt, stream};
use le_rust::chat::{ChatRequest, Message};
use le_rust::fim::FimRequest;
use le_rust::models::ListModelsOptions;
use le_rust::{Client, ErrorKind, Limits, RequestOptions, TimeoutStage};
use serde_json::{Value, json};
use tokio::net::TcpListener;
use tokio::sync::Mutex;

#[derive(Debug, Clone)]
struct Captured {
    method: String,
    uri: String,
    authorization: Option<String>,
    body: Value,
}

#[derive(Default)]
struct TestState {
    requests: Mutex<Vec<Captured>>,
}

async fn api_handler(
    State(state): State<Arc<TestState>>,
    request: Request<Body>,
) -> Response<Body> {
    let method = request.method().to_string();
    let uri = request.uri().to_string();
    let authorization = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let body_bytes = to_bytes(request.into_body(), 1024 * 1024).await.unwrap();
    let body = if body_bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&body_bytes).unwrap()
    };
    state.requests.lock().await.push(Captured {
        method,
        uri: uri.clone(),
        authorization,
        body: body.clone(),
    });

    match uri.split('?').next().unwrap() {
        "/v1/models" => json_response(json!({
            "object": "list",
            "data": [{
                "id": "mistral-test",
                "object": "model",
                "capabilities": { "completion_chat": true, "future_capability": "kept" },
                "future_field": 7
            }]
        })),
        "/v1/models/mistral-test" => json_response(json!({
            "id": "mistral-test",
            "object": "model",
            "capabilities": { "completion_chat": true }
        })),
        "/v1/chat/completions" if body["stream"] == true => stream_response(vec![
            "data: {\"id\":\"chat-1\",\"model\":\"mistral-test\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\",\"tool_calls\":[{\"index\":0,\"id\":\"call-1\",\"type\":\"function\",\"function\":{\"name\":\"weather\",\"arguments\":\"{\\\"ci\"}}]}}]}\r\n\r\n",
            "data: {\"id\":\"chat-1\",\"model\":\"mistral-test\",\"choices\":[{\"index\":0,\"delta\":{\"tool_calls\":[{\"index\":0,\"function\":{\"arguments\":\"ty\\\":\\\"Vienna\\\"}\"}}]},\"finish_reason\":\"tool_calls\"}],\"usage\":{\"prompt_tokens\":4,\"completion_tokens\":2,\"total_tokens\":6}}\n\n",
            "data: [DONE]\n\n",
        ]),
        "/v1/chat/completions" => json_response(json!({
            "id": "chat-1",
            "model": "mistral-test",
            "choices": [{
                "index": 0,
                "message": { "role": "assistant", "content": "hello", "future_block": true },
                "finish_reason": "future_reason"
            }],
            "usage": { "prompt_tokens": 2, "completion_tokens": 1, "total_tokens": 3 }
        })),
        "/v1/fim/completions" if body["stream"] == true => stream_response(vec![
            "data: {\"id\":\"fim-1\",\"model\":\"codestral-test\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"middle\"},\"finish_reason\":\"stop\"}]}\n\n",
            "data: [DONE]\n\n",
        ]),
        "/v1/fim/completions" => json_response(json!({
            "id": "fim-1",
            "model": "codestral-test",
            "choices": [{
                "index": 0,
                "message": { "role": "assistant", "content": "middle" },
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
        .header("x-request-id", "req-test")
        .body(Body::from(value.to_string()))
        .unwrap()
}

fn stream_response(parts: Vec<&'static str>) -> Response<Body> {
    let chunks = stream::iter(
        parts
            .into_iter()
            .map(|part| Ok::<Bytes, Infallible>(Bytes::from_static(part.as_bytes()))),
    );
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/event-stream; charset=utf-8")
        .header("x-request-id", "req-stream")
        .body(Body::from_stream(chunks))
        .unwrap()
}

async fn spawn_api() -> (String, Arc<TestState>) {
    let state = Arc::new(TestState::default());
    let app = Router::new()
        .fallback(api_handler)
        .with_state(state.clone());
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (format!("http://{address}/v1/"), state)
}

fn test_client(base_url: String) -> Client {
    Client::builder()
        .api_key("test-secret")
        .base_url(base_url)
        .allow_insecure_loopback(true)
        .build()
        .unwrap()
}

fn test_client_with_limits(base_url: String, limits: Limits) -> Client {
    Client::builder()
        .api_key("test-secret")
        .base_url(base_url)
        .allow_insecure_loopback(true)
        .limits(limits)
        .build()
        .unwrap()
}

#[tokio::test]
async fn supported_methods_round_trip_through_public_api() {
    let (base_url, state) = spawn_api().await;
    let client = test_client(base_url);

    let models = client
        .models()
        .list(ListModelsOptions {
            provider: Some("mistral".into()),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(models.meta.request_id.as_deref(), Some("req-test"));
    assert_eq!(models.data.data[0].id, "mistral-test");
    assert_eq!(models.data.data[0].extra["future_field"], 7);
    assert_eq!(
        models.data.data[0].capabilities.extra["future_capability"],
        "kept"
    );

    let model = client
        .models()
        .retrieve("mistral-test", RequestOptions::default())
        .await
        .unwrap();
    assert_eq!(model.data.id, "mistral-test");

    let chat = client
        .chat()
        .complete(
            ChatRequest::new("mistral-test", vec![Message::user("hello")]),
            RequestOptions::default(),
        )
        .await
        .unwrap();
    assert_eq!(
        chat.data.choices[0]
            .message()
            .and_then(|message| message.text()),
        Some("hello")
    );
    assert_eq!(
        chat.data.choices[0]
            .finish_reason
            .as_ref()
            .unwrap()
            .as_str(),
        "future_reason"
    );

    let streamed = client
        .chat()
        .stream(
            ChatRequest::new("mistral-test", vec![Message::user("weather")]),
            RequestOptions::default(),
        )
        .await
        .unwrap()
        .collect_response()
        .await
        .unwrap();
    let arguments: Value = streamed.choices[0].message().unwrap().tool_calls[0]
        .function
        .parse_arguments()
        .unwrap();
    assert_eq!(arguments, json!({"city":"Vienna"}));
    assert_eq!(streamed.usage.unwrap().total_tokens, Some(6));

    let fim = client
        .fim()
        .complete(
            FimRequest::new("codestral-test", "before").suffix("after"),
            RequestOptions::default(),
        )
        .await
        .unwrap();
    assert_eq!(
        fim.data.choices[0]
            .message()
            .and_then(|message| message.text()),
        Some("middle")
    );

    let fim_stream = client
        .fim()
        .stream(
            FimRequest::new("codestral-test", "before").suffix("after"),
            RequestOptions::default(),
        )
        .await
        .unwrap()
        .collect_response()
        .await
        .unwrap();
    assert_eq!(
        fim_stream.choices[0]
            .message()
            .and_then(|message| message.text()),
        Some("middle")
    );

    let requests = state.requests.lock().await;
    assert_eq!(requests.len(), 6);
    assert!(
        requests
            .iter()
            .all(|request| request.method == "GET" || request.method == "POST")
    );
    assert!(
        requests
            .iter()
            .all(|request| request.authorization.as_deref() == Some("Bearer test-secret"))
    );
    assert!(requests[0].uri.contains("provider=mistral"));
    assert!(requests[2].body["stream"] == false);
    assert!(requests[3].body["stream"] == true);
    assert!(requests[4].body["suffix"] == "after");
}

#[tokio::test]
async fn cloned_client_supports_concurrent_requests() {
    let (base_url, state) = spawn_api().await;
    let client = test_client(base_url);
    let requests = (0..24).map(|_| {
        let client = client.clone();
        async move {
            client
                .models()
                .list(ListModelsOptions::default())
                .await
                .unwrap()
                .data
                .data[0]
                .id
                .clone()
        }
    });

    let models = futures_util::future::join_all(requests).await;
    assert!(models.iter().all(|model| model == "mistral-test"));
    assert_eq!(state.requests.lock().await.len(), 24);
}

#[tokio::test]
async fn premature_eof_is_terminal_error() {
    async fn truncated() -> Response<Body> {
        stream_response(vec![
            "data: {\"id\":\"x\",\"model\":\"m\",\"choices\":[]}\n\n",
        ])
    }
    let app = Router::new().fallback(truncated);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let client = test_client(format!("http://{address}/v1/"));
    let mut response = client
        .chat()
        .stream(
            ChatRequest::new("m", vec![Message::user("x")]),
            RequestOptions::default(),
        )
        .await
        .unwrap();
    assert!(response.next().await.unwrap().is_ok());
    let error = response.next().await.unwrap().unwrap_err();
    assert_eq!(error.kind(), ErrorKind::UnexpectedEof);
    assert!(response.next().await.is_none());
}

#[tokio::test]
async fn http_failures_are_not_retried_or_exposed_by_default() {
    async fn fail(State(calls): State<Arc<AtomicUsize>>) -> Response<Body> {
        calls.fetch_add(1, Ordering::SeqCst);
        Response::builder()
            .status(StatusCode::TOO_MANY_REQUESTS)
            .header(header::CONTENT_TYPE, "application/json")
            .header("retry-after-ms", "250")
            .body(Body::from(
                json!({"type":"rate_limit", "message":"prompt-secret"}).to_string(),
            ))
            .unwrap()
    }
    let calls = Arc::new(AtomicUsize::new(0));
    let app = Router::new().fallback(fail).with_state(calls.clone());
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let client = test_client(format!("http://{address}/v1/"));
    let error = client
        .chat()
        .complete(
            ChatRequest::new("m", vec![Message::user("prompt-secret")]),
            RequestOptions::default(),
        )
        .await
        .unwrap_err();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(!format!("{error:?}").contains("prompt-secret"));
    let le_rust::Error::HttpApi(api) = error else {
        panic!("expected API error");
    };
    assert_eq!(api.retry_after, Some(std::time::Duration::from_millis(250)));
    assert!(api.diagnostic_body().contains("prompt-secret"));
}

#[tokio::test]
async fn model_ids_are_encoded_as_one_path_segment() {
    let (base_url, state) = spawn_api().await;
    let client = test_client(base_url);
    let error = client
        .models()
        .retrieve("org/model ?#", RequestOptions::default())
        .await
        .unwrap_err();
    assert_eq!(error.kind(), ErrorKind::HttpApi);

    let requests = state.requests.lock().await;
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].uri, "/v1/models/org%2Fmodel%20%3F%23");
}

#[tokio::test]
async fn complete_response_body_is_bounded() {
    async fn oversized() -> Response<Body> {
        json_response(json!({"object":"list", "data":[], "padding":"x".repeat(256)}))
    }
    let app = Router::new().fallback(oversized);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let mut limits = Limits::default();
    limits.response_body_bytes = 64;
    let client = test_client_with_limits(format!("http://{address}/v1/"), limits);

    let error = client
        .models()
        .list(ListModelsOptions::default())
        .await
        .unwrap_err();
    assert_eq!(error.kind(), ErrorKind::LimitExceeded);
}

#[tokio::test]
async fn provider_error_body_is_truncated_and_explicit_only() {
    async fn fail() -> Response<Body> {
        Response::builder()
            .status(StatusCode::UNPROCESSABLE_ENTITY)
            .header("x-request-id", "req-error")
            .body(Body::from("sensitive-provider-body"))
            .unwrap()
    }
    let app = Router::new().fallback(fail);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let mut limits = Limits::default();
    limits.error_body_bytes = 9;
    let client = test_client_with_limits(format!("http://{address}/v1/"), limits);

    let error = client
        .models()
        .list(ListModelsOptions::default())
        .await
        .unwrap_err();
    assert!(!format!("{error:?}").contains("sensitive"));
    let le_rust::Error::HttpApi(api) = error else {
        panic!("expected API error");
    };
    assert_eq!(api.request_id.as_deref(), Some("req-error"));
    assert!(api.body_truncated());
    assert_eq!(api.diagnostic_body(), "sensitive");
}

#[tokio::test]
async fn successful_stream_requires_event_stream_content_type() {
    async fn json_instead_of_sse() -> Response<Body> {
        json_response(json!({"unexpected": true}))
    }
    let app = Router::new().fallback(json_instead_of_sse);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let client = test_client(format!("http://{address}/v1/"));

    let error = client
        .chat()
        .stream(
            ChatRequest::new("m", vec![Message::user("x")]),
            RequestOptions::default(),
        )
        .await
        .unwrap_err();
    assert_eq!(error.kind(), ErrorKind::StreamProtocol);
}

#[tokio::test]
async fn malformed_event_is_terminal_even_when_later_events_are_queued() {
    async fn malformed_then_valid() -> Response<Body> {
        stream_response(vec![
            "data: not-json\n\ndata: {\"id\":\"x\",\"model\":\"m\",\"choices\":[]}\n\ndata: [DONE]\n\n",
        ])
    }
    let app = Router::new().fallback(malformed_then_valid);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let client = test_client(format!("http://{address}/v1/"));
    let mut response = client
        .chat()
        .stream(
            ChatRequest::new("m", vec![Message::user("x")]),
            RequestOptions::default(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.next().await.unwrap().unwrap_err().kind(),
        ErrorKind::StreamProtocol
    );
    assert!(response.next().await.is_none());
}

#[tokio::test]
async fn done_finishes_without_waiting_for_connection_close() {
    async fn done_then_pending() -> Response<Body> {
        let done = stream::once(async {
            Ok::<Bytes, Infallible>(Bytes::from_static(b"data: [DONE]\n\n"))
        });
        let never = stream::pending::<Result<Bytes, Infallible>>();
        Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "text/event-stream")
            .body(Body::from_stream(done.chain(never)))
            .unwrap()
    }
    let app = Router::new().fallback(done_then_pending);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let client = test_client(format!("http://{address}/v1/"));
    let mut response = client
        .chat()
        .stream(
            ChatRequest::new("m", vec![Message::user("x")]),
            RequestOptions::default(),
        )
        .await
        .unwrap();

    let end = tokio::time::timeout(Duration::from_millis(100), response.next())
        .await
        .expect("stream should finish at DONE");
    assert!(end.is_none());
    assert!(response.protocol_complete());
}

#[test]
fn core_runtime_types_have_expected_thread_bounds() {
    fn assert_send_sync<T: Send + Sync>() {}
    fn assert_send<T: Send>() {}

    assert_send_sync::<Client>();
    assert_send::<le_rust::CompletionStream>();
}

#[tokio::test]
async fn response_header_deadline_is_classified() {
    async fn delayed() -> Response<Body> {
        tokio::time::sleep(Duration::from_millis(100)).await;
        json_response(json!({"object":"list", "data":[]}))
    }
    let app = Router::new().fallback(delayed);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let client = Client::builder()
        .api_key("test-secret")
        .base_url(format!("http://{address}/v1/"))
        .allow_insecure_loopback(true)
        .header_timeout(Duration::from_millis(10))
        .build()
        .unwrap();

    let error = client
        .models()
        .list(ListModelsOptions::default())
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        le_rust::Error::Timeout {
            stage: TimeoutStage::ResponseHeaders
        }
    ));
}

#[tokio::test]
async fn response_body_deadline_is_classified() {
    async fn headers_then_stall() -> Response<Body> {
        let prefix = stream::once(async {
            Ok::<Bytes, Infallible>(Bytes::from_static(b"{\"object\":\"list\","))
        });
        let never = stream::pending::<Result<Bytes, Infallible>>();
        Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from_stream(prefix.chain(never)))
            .unwrap()
    }
    let app = Router::new().fallback(headers_then_stall);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let client = Client::builder()
        .api_key("test-secret")
        .base_url(format!("http://{address}/v1/"))
        .allow_insecure_loopback(true)
        .operation_timeout(Duration::from_millis(20))
        .build()
        .unwrap();

    let error = client
        .models()
        .list(ListModelsOptions::default())
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        le_rust::Error::Timeout {
            stage: TimeoutStage::ResponseBody
        }
    ));
}

#[tokio::test]
async fn per_request_options_may_only_narrow_client_defaults() {
    let (base_url, state) = spawn_api().await;
    let client = test_client(base_url);

    for request in [
        RequestOptions::default().operation_timeout(Duration::ZERO),
        RequestOptions::default().header_timeout(Duration::from_secs(61)),
        RequestOptions::default().response_body_bytes(16 * 1024 * 1024 + 1),
    ] {
        let error = client
            .models()
            .retrieve("mistral-test", request)
            .await
            .unwrap_err();
        assert_eq!(error.kind(), ErrorKind::InvalidRequest);
    }
    assert!(state.requests.lock().await.is_empty());
}

#[derive(Clone)]
struct RedirectState {
    location: String,
    calls: Arc<AtomicUsize>,
}

async fn redirect(State(state): State<RedirectState>) -> Response<Body> {
    state.calls.fetch_add(1, Ordering::SeqCst);
    Response::builder()
        .status(StatusCode::TEMPORARY_REDIRECT)
        .header(header::LOCATION, state.location)
        .body(Body::empty())
        .unwrap()
}

async fn redirect_target(State(calls): State<Arc<AtomicUsize>>) -> Response<Body> {
    calls.fetch_add(1, Ordering::SeqCst);
    json_response(json!({"object":"list", "data":[]}))
}

#[tokio::test]
async fn redirects_are_not_followed_to_another_origin() {
    let target_calls = Arc::new(AtomicUsize::new(0));
    let target = Router::new()
        .fallback(redirect_target)
        .with_state(target_calls.clone());
    let target_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let target_address = target_listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(target_listener, target).await.unwrap() });

    let origin_calls = Arc::new(AtomicUsize::new(0));
    let origin = Router::new().fallback(redirect).with_state(RedirectState {
        location: format!("http://{target_address}/stolen"),
        calls: origin_calls.clone(),
    });
    let origin_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin_address = origin_listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(origin_listener, origin).await.unwrap() });
    let client = test_client(format!("http://{origin_address}/v1/"));

    let error = client
        .models()
        .list(ListModelsOptions::default())
        .await
        .unwrap_err();
    let le_rust::Error::HttpApi(api) = error else {
        panic!("expected redirect response to remain an API error");
    };
    assert_eq!(api.status, StatusCode::TEMPORARY_REDIRECT);
    assert_eq!(origin_calls.load(Ordering::SeqCst), 1);
    assert_eq!(target_calls.load(Ordering::SeqCst), 0);
}
