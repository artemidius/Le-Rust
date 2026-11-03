use std::collections::{BTreeMap, VecDeque};
use std::pin::Pin;
use std::task::{Context, Poll};

use bytes::Bytes;
use futures_core::Stream;
use futures_util::StreamExt;

use crate::chat::{
    AssistantResponseMessage, ChatChoice, ChatCompletionResponse, CompletionChunk, Content,
    FinishReason, FunctionCall, ToolArguments, ToolCall, ToolCallDelta,
};
use crate::{Error, Limits, ResponseMeta, Result, Usage};

type ByteStream = Pin<Box<dyn Stream<Item = std::result::Result<Bytes, reqwest::Error>> + Send>>;

/// Incremental Mistral completion stream.
///
/// The stream succeeds only after receiving the provider's `[DONE]` marker.
/// Dropping the owned value stops local consumption but cannot promise remote
/// cancellation or avoided billing.
pub struct CompletionStream {
    source: ByteStream,
    parser: SseParser,
    ready: VecDeque<Result<CompletionChunk>>,
    terminal: bool,
    protocol_complete: bool,
    meta: ResponseMeta,
    limits: Limits,
}

impl std::fmt::Debug for CompletionStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompletionStream")
            .field("terminal", &self.terminal)
            .field("protocol_complete", &self.protocol_complete)
            .field("meta", &self.meta)
            .finish_non_exhaustive()
    }
}

impl CompletionStream {
    pub(crate) fn new<S>(source: S, meta: ResponseMeta, limits: Limits) -> Self
    where
        S: Stream<Item = std::result::Result<Bytes, reqwest::Error>> + Send + 'static,
    {
        Self {
            source: Box::pin(source),
            parser: SseParser::new(
                limits.sse_line_bytes,
                limits.sse_event_bytes,
                limits.sse_queued_events,
            ),
            ready: VecDeque::new(),
            terminal: false,
            protocol_complete: false,
            meta,
            limits,
        }
    }

    pub fn response_meta(&self) -> &ResponseMeta {
        &self.meta
    }

    pub fn protocol_complete(&self) -> bool {
        self.protocol_complete
    }

    pub async fn collect_response(mut self) -> Result<ChatCompletionResponse> {
        let mut collector = StreamCollector::new(self.limits);
        while let Some(chunk) = self.next().await {
            collector.push(chunk?)?;
        }
        if !self.protocol_complete {
            return Err(Error::UnexpectedEof);
        }
        collector.finish()
    }

    fn enqueue_events(&mut self) -> Result<()> {
        while let Some(data) = self.parser.events.pop_front() {
            if data == b"[DONE]" {
                self.protocol_complete = true;
                self.terminate();
                break;
            }
            let chunk = serde_json::from_slice(&data)
                .map_err(|error| Error::StreamProtocol(error.to_string()));
            self.ready.push_back(chunk);
        }
        Ok(())
    }

    fn terminate(&mut self) {
        self.terminal = true;
        // Drop the response body now rather than retaining its connection until
        // the caller drops an already exhausted CompletionStream.
        self.source = Box::pin(futures_util::stream::empty());
    }
}

impl Stream for CompletionStream {
    type Item = Result<CompletionChunk>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            if let Some(item) = self.ready.pop_front() {
                if item.is_err() {
                    self.ready.clear();
                    self.terminate();
                }
                return Poll::Ready(Some(item));
            }
            if self.terminal {
                return Poll::Ready(None);
            }
            match self.source.as_mut().poll_next(cx) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(Some(Ok(bytes))) => {
                    if let Err(error) = self.parser.feed(&bytes) {
                        self.terminate();
                        return Poll::Ready(Some(Err(error)));
                    }
                    if let Err(error) = self.enqueue_events() {
                        self.terminate();
                        return Poll::Ready(Some(Err(error)));
                    }
                }
                Poll::Ready(Some(Err(error))) => {
                    self.terminate();
                    let error = if error.is_timeout() {
                        Error::Timeout {
                            stage: crate::TimeoutStage::ResponseBody,
                        }
                    } else {
                        Error::Transport(error)
                    };
                    return Poll::Ready(Some(Err(error)));
                }
                Poll::Ready(None) => {
                    self.terminate();
                    if self.protocol_complete {
                        return Poll::Ready(None);
                    }
                    return Poll::Ready(Some(Err(Error::UnexpectedEof)));
                }
            }
        }
    }
}

struct SseParser {
    buffer: Vec<u8>,
    data_lines: Vec<Vec<u8>>,
    event_bytes: usize,
    first_line: bool,
    line_limit: usize,
    event_limit: usize,
    queue_limit: usize,
    events: VecDeque<Vec<u8>>,
}

impl SseParser {
    fn new(line_limit: usize, event_limit: usize, queue_limit: usize) -> Self {
        Self {
            buffer: Vec::new(),
            data_lines: Vec::new(),
            event_bytes: 0,
            first_line: true,
            line_limit,
            event_limit,
            queue_limit,
            events: VecDeque::new(),
        }
    }

    fn feed(&mut self, bytes: &[u8]) -> Result<()> {
        self.buffer.extend_from_slice(bytes);
        loop {
            let Some((position, delimiter)) = find_line_end(&self.buffer) else {
                if self.buffer.len() > self.line_limit {
                    return Err(Error::LimitExceeded {
                        resource: "SSE line",
                        limit: self.line_limit,
                    });
                }
                return Ok(());
            };
            if position > self.line_limit {
                return Err(Error::LimitExceeded {
                    resource: "SSE line",
                    limit: self.line_limit,
                });
            }
            let mut line = self
                .buffer
                .drain(..position + delimiter)
                .collect::<Vec<_>>();
            line.truncate(position);
            if self.first_line {
                self.first_line = false;
                if line.starts_with(&[0xEF, 0xBB, 0xBF]) {
                    line.drain(..3);
                }
            }
            self.process_line(&line)?;
        }
    }

    fn process_line(&mut self, line: &[u8]) -> Result<()> {
        if line.is_empty() {
            if !self.data_lines.is_empty() {
                let mut event = Vec::with_capacity(self.event_bytes);
                for (index, data) in self.data_lines.drain(..).enumerate() {
                    if index > 0 {
                        event.push(b'\n');
                    }
                    event.extend_from_slice(&data);
                }
                std::str::from_utf8(&event)
                    .map_err(|_| Error::StreamProtocol("SSE data was not valid UTF-8".into()))?;
                if self.events.len() >= self.queue_limit {
                    return Err(Error::LimitExceeded {
                        resource: "queued SSE events",
                        limit: self.queue_limit,
                    });
                }
                self.events.push_back(event);
            }
            self.event_bytes = 0;
            return Ok(());
        }
        if line.starts_with(b":") {
            return Ok(());
        }
        let (field, mut value) = match line.iter().position(|byte| *byte == b':') {
            Some(position) => (&line[..position], &line[position + 1..]),
            None => (line, &[][..]),
        };
        if value.first() == Some(&b' ') {
            value = &value[1..];
        }
        if field == b"data" {
            let next_size = self
                .event_bytes
                .saturating_add(value.len())
                .saturating_add(usize::from(!self.data_lines.is_empty()));
            if next_size > self.event_limit {
                return Err(Error::LimitExceeded {
                    resource: "SSE event",
                    limit: self.event_limit,
                });
            }
            self.event_bytes = next_size;
            self.data_lines.push(value.to_vec());
        }
        Ok(())
    }
}

fn find_line_end(buffer: &[u8]) -> Option<(usize, usize)> {
    for (index, byte) in buffer.iter().enumerate() {
        match byte {
            b'\n' => return Some((index, 1)),
            b'\r' if index + 1 < buffer.len() => {
                return Some((index, usize::from(buffer[index + 1] == b'\n') + 1));
            }
            b'\r' => return None,
            _ => {}
        }
    }
    None
}

/// Bounded helper for assembling a completed response from native stream chunks.
pub struct StreamCollector {
    limits: Limits,
    id: Option<String>,
    model: Option<String>,
    object: Option<String>,
    created: Option<u64>,
    usage: Option<Usage>,
    choices: BTreeMap<usize, ChoiceAccumulator>,
    bytes: usize,
}

impl StreamCollector {
    pub fn new(limits: Limits) -> Self {
        Self {
            limits,
            id: None,
            model: None,
            object: None,
            created: None,
            usage: None,
            choices: BTreeMap::new(),
            bytes: 0,
        }
    }

    pub fn push(&mut self, chunk: CompletionChunk) -> Result<()> {
        set_consistent(&mut self.id, chunk.id, "completion ID")?;
        set_consistent(&mut self.model, chunk.model, "model")?;
        set_consistent(&mut self.object, chunk.object, "object")?;
        if let Some(created) = chunk.created {
            if self.created.is_some_and(|current| current != created) {
                return Err(Error::Accumulation("conflicting creation time".into()));
            }
            self.created = Some(created);
        }
        if chunk.usage.is_some() {
            self.usage = chunk.usage;
        }
        for choice in chunk.choices {
            let target = self.choices.entry(choice.index).or_default();
            if let Some(role) = choice.delta.role {
                set_consistent(&mut target.role, Some(role), "message role")?;
            }
            if let Some(content) = choice.delta.content {
                self.bytes = self.bytes.saturating_add(content_size(&content));
                if self.bytes > self.limits.accumulated_completion_bytes {
                    return Err(Error::LimitExceeded {
                        resource: "accumulated completion",
                        limit: self.limits.accumulated_completion_bytes,
                    });
                }
                append_content(&mut target.content, content)?;
            }
            for tool in choice.delta.tool_calls {
                target.push_tool(tool, self.limits.tool_arguments_bytes)?;
            }
            if let Some(reason) = choice.finish_reason {
                if target
                    .finish_reason
                    .as_ref()
                    .is_some_and(|old| old != &reason)
                {
                    return Err(Error::Accumulation(
                        "conflicting finish reason for one choice".into(),
                    ));
                }
                target.finish_reason = Some(reason);
            }
        }
        Ok(())
    }

    pub fn finish(self) -> Result<ChatCompletionResponse> {
        let id = self
            .id
            .ok_or_else(|| Error::Accumulation("stream did not identify the completion".into()))?;
        let model = self
            .model
            .ok_or_else(|| Error::Accumulation("stream did not identify the model".into()))?;
        let mut choices = Vec::with_capacity(self.choices.len());
        for (index, choice) in self.choices {
            choices.push(choice.finish(index)?);
        }
        Ok(ChatCompletionResponse {
            id,
            model,
            object: self.object,
            created: self.created,
            choices,
            usage: self.usage,
            extra: BTreeMap::new(),
        })
    }
}

#[derive(Default)]
struct ChoiceAccumulator {
    role: Option<String>,
    content: Option<Content>,
    tools: BTreeMap<usize, ToolAccumulator>,
    finish_reason: Option<FinishReason>,
}

impl ChoiceAccumulator {
    fn push_tool(&mut self, delta: ToolCallDelta, limit: usize) -> Result<()> {
        let index = delta.index.ok_or_else(|| {
            Error::Accumulation("streamed tool call did not include an index".into())
        })?;
        self.tools.entry(index).or_default().push(delta, limit)
    }

    fn finish(self, index: usize) -> Result<ChatChoice> {
        let mut tool_calls = Vec::with_capacity(self.tools.len());
        for (tool_index, tool) in self.tools {
            tool_calls.push(tool.finish(tool_index)?);
        }
        Ok(ChatChoice {
            index,
            message: Some(AssistantResponseMessage {
                role: self.role.or_else(|| Some("assistant".into())),
                content: self.content,
                tool_calls,
                prefix: None,
                extra: BTreeMap::new(),
            }),
            messages: Vec::new(),
            finish_reason: self.finish_reason,
            extra: BTreeMap::new(),
        })
    }
}

#[derive(Default)]
struct ToolAccumulator {
    id: Option<String>,
    kind: Option<String>,
    name: Option<String>,
    arguments: Option<ToolArguments>,
}

impl ToolAccumulator {
    fn push(&mut self, delta: ToolCallDelta, limit: usize) -> Result<()> {
        set_consistent(&mut self.id, delta.id, "tool call ID")?;
        set_consistent(&mut self.kind, delta.kind, "tool call type")?;
        if let Some(function) = delta.function {
            set_consistent(&mut self.name, function.name, "tool function name")?;
            if let Some(arguments) = function.arguments {
                match (&mut self.arguments, arguments) {
                    (None, arguments) => self.arguments = Some(arguments),
                    (Some(ToolArguments::String(current)), ToolArguments::String(fragment)) => {
                        if current.len().saturating_add(fragment.len()) > limit {
                            return Err(Error::LimitExceeded {
                                resource: "tool arguments",
                                limit,
                            });
                        }
                        current.push_str(&fragment);
                    }
                    (Some(ToolArguments::Json(current)), ToolArguments::Json(next))
                        if *current == next => {}
                    _ => {
                        return Err(Error::Accumulation(
                            "conflicting tool argument representations".into(),
                        ));
                    }
                }
            }
        }
        let size = match &self.arguments {
            Some(ToolArguments::String(value)) => value.len(),
            Some(ToolArguments::Json(value)) => value.to_string().len(),
            None => 0,
        };
        if size > limit {
            return Err(Error::LimitExceeded {
                resource: "tool arguments",
                limit,
            });
        }
        Ok(())
    }

    fn finish(self, index: usize) -> Result<ToolCall> {
        Ok(ToolCall {
            id: self.id,
            kind: self.kind.or_else(|| Some("function".into())),
            function: FunctionCall {
                name: self.name.ok_or_else(|| {
                    Error::Accumulation("tool call did not include a function name".into())
                })?,
                arguments: self.arguments.ok_or_else(|| {
                    Error::Accumulation("tool call did not include arguments".into())
                })?,
                extra: BTreeMap::new(),
            },
            index: Some(index),
            extra: BTreeMap::new(),
        })
    }
}

fn set_consistent(target: &mut Option<String>, next: Option<String>, field: &str) -> Result<()> {
    if let Some(next) = next {
        if target.as_ref().is_some_and(|current| current != &next) {
            return Err(Error::Accumulation(format!("conflicting {field}")));
        }
        *target = Some(next);
    }
    Ok(())
}

fn content_size(content: &Content) -> usize {
    match content {
        Content::Text(text) => text.len(),
        Content::Blocks(blocks) => blocks.iter().map(|block| block.to_string().len()).sum(),
    }
}

fn append_content(target: &mut Option<Content>, next: Content) -> Result<()> {
    match (target, next) {
        (slot @ None, content) => *slot = Some(content),
        (Some(Content::Text(current)), Content::Text(fragment)) => current.push_str(&fragment),
        (Some(Content::Blocks(current)), Content::Blocks(mut blocks)) => {
            current.append(&mut blocks)
        }
        _ => {
            return Err(Error::Accumulation(
                "stream changed content representation".into(),
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parser_handles_crlf_split_and_multiline_data() {
        let mut parser = SseParser::new(1024, 1024, 16);
        parser.feed(b"data: {\"a\":\r").unwrap();
        parser.feed(b"\ndata: 1}\r\n\r").unwrap();
        parser.feed(b"\n").unwrap();
        assert_eq!(parser.events.pop_front().unwrap(), b"{\"a\":\n1}");
    }

    #[test]
    fn parser_handles_every_two_part_split() {
        let input = "data: {\"choices\":[],\"text\":\"héllo\"}\n\ndata: [DONE]\n\n".as_bytes();
        for split in 0..=input.len() {
            let mut parser = SseParser::new(4096, 4096, 16);
            parser.feed(&input[..split]).unwrap();
            parser.feed(&input[split..]).unwrap();
            assert_eq!(parser.events.len(), 2, "split at {split}");
        }
    }

    #[test]
    fn parser_handles_one_byte_chunks_and_bom() {
        let input = b"\xEF\xBB\xBF: comment\r\ndata: {\"text\":\"h\xC3\xA9\"}\r\n\r\n";
        let mut parser = SseParser::new(4096, 4096, 16);
        for byte in input {
            parser.feed(std::slice::from_ref(byte)).unwrap();
        }
        assert_eq!(
            parser.events.pop_front().unwrap(),
            "{\"text\":\"hé\"}".as_bytes()
        );
    }

    #[test]
    fn parser_rejects_oversized_unterminated_line() {
        let mut parser = SseParser::new(3, 100, 16);
        let error = parser.feed(b"data").unwrap_err();
        assert_eq!(error.kind(), crate::ErrorKind::LimitExceeded);
    }

    #[test]
    fn parser_bounds_events_queued_from_one_chunk() {
        let mut parser = SseParser::new(1024, 1024, 1);
        let error = parser
            .feed(b"data: {\"choices\":[]}\n\ndata: {\"choices\":[]}\n\n")
            .unwrap_err();
        assert_eq!(error.kind(), crate::ErrorKind::LimitExceeded);
    }
}
