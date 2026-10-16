//! Models that run on the local ComfyUI server, which `comfyui_launcher` starts on demand. Each
//! model is a bundled API-format workflow under `comfyui_workflows/` plus a function that fills in
//! the prompt box's values.

pub mod comfyui_launcher;
pub mod generate_with_comfyui;
pub mod minimax_h3_workflow;
pub mod qwen_image_workflow;

pub const COMFYUI_HOST: &str = "http://127.0.0.1:8188";

pub const MINIMAX_H3_MODEL: &str = "comfyui/minimax-h3";
pub const QWEN_IMAGE_MODEL: &str = "comfyui/qwen-image-2.1";
