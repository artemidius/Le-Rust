#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]

mod client;
mod error;
mod stream;

pub mod chat;
pub mod fim;
pub mod models;
pub mod types;

pub use client::{Client, ClientBuilder, Limits, RequestOptions};
pub use error::{ApiError, Error, ErrorKind, Result, TimeoutStage};
pub use stream::{CompletionStream, StreamCollector};
pub use types::{ApiResponse, ResponseMeta, Usage};
