//! Client for a local ComfyUI server: queue an API-format workflow, follow it, fetch what it saved.
//! Routes: <https://docs.comfy.org/development/comfyui-server/comms_routes>

pub mod error;
pub mod files;
pub mod prompts;
pub mod server;

mod http;
