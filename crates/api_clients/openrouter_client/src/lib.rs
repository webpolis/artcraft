//! Client for OpenRouter's asynchronous video generation API: submit a job, poll it, download
//! the result. Docs: <https://openrouter.ai/docs/guides/overview/multimodal/video-generation>

pub mod error;
pub mod videos;

mod http;
