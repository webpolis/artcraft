use std::collections::BTreeMap;

use log::info;
use serde_derive::Deserialize;
use serde_json::{json, Value};

use crate::error::ComfyuiError;
use crate::files::FileRef;
use crate::http::read_json;

const CLIENT_ID: &str = "artcraft";

/// Where a queued workflow stands.
#[derive(Debug, PartialEq)]
pub enum PromptState {
  /// Waiting in the queue or running.
  Queued,
  /// Finished; every file the workflow's save nodes wrote.
  Finished(Vec<FileRef>),
  /// A node raised an error.
  Failed(String),
  /// Neither queued nor in the history, e.g. ComfyUI restarted mid-run.
  Missing,
}

#[derive(Deserialize)]
struct QueuePromptResponse {
  prompt_id: String,
}

#[derive(Deserialize)]
struct HistoryEntry {
  #[serde(default)]
  outputs: BTreeMap<String, NodeOutput>,
  status: HistoryStatus,
}

#[derive(Deserialize)]
struct NodeOutput {
  /// Images and videos alike: SaveVideo reports under `images` too.
  #[serde(default)]
  images: Vec<FileRef>,
}

#[derive(Deserialize)]
struct HistoryStatus {
  status_str: String,
  /// `[event, data]` pairs, e.g. `["execution_error", {"node_type": ..., "exception_message": ...}]`.
  #[serde(default)]
  messages: Vec<(String, Value)>,
}

#[derive(Deserialize)]
struct QueueResponse {
  /// Entries are `[number, prompt_id, prompt, extra_data, outputs]`.
  queue_running: Vec<Vec<Value>>,
  queue_pending: Vec<Vec<Value>>,
}

/// Queues an API-format workflow and returns its prompt id.
pub async fn queue_prompt(host: &str, workflow: &Value) -> Result<String, ComfyuiError> {
  let response = reqwest::Client::new()
    .post(format!("{}/prompt", host))
    .json(&json!({ "prompt": workflow, "client_id": CLIENT_ID }))
    .send()
    .await?;
  let prompt_id = read_json::<QueuePromptResponse>(response).await?.prompt_id;
  info!("ComfyUI queued prompt {}", prompt_id);
  Ok(prompt_id)
}

pub async fn get_prompt_state(host: &str, prompt_id: &str) -> Result<PromptState, ComfyuiError> {
  let response = reqwest::Client::new().get(format!("{}/history/{}", host, prompt_id)).send().await?;
  let mut history: BTreeMap<String, HistoryEntry> = read_json(response).await?;
  if let Some(entry) = history.remove(prompt_id) {
    return Ok(state_from_history(entry));
  }
  let response = reqwest::Client::new().get(format!("{}/queue", host)).send().await?;
  let queue: QueueResponse = read_json(response).await?;
  Ok(if is_queued(&queue, prompt_id) { PromptState::Queued } else { PromptState::Missing })
}

fn state_from_history(entry: HistoryEntry) -> PromptState {
  if entry.status.status_str == "error" {
    return PromptState::Failed(failure_message(&entry.status.messages));
  }
  PromptState::Finished(entry.outputs.into_values().flat_map(|output| output.images).collect())
}

fn failure_message(messages: &[(String, Value)]) -> String {
  messages.iter()
    .find(|(event, _)| event == "execution_error")
    .map(|(_, data)| format!(
      "{} failed: {}",
      data["node_type"].as_str().unwrap_or("A node"),
      data["exception_message"].as_str().unwrap_or("unknown error").trim(),
    ))
    .unwrap_or_else(|| "The ComfyUI workflow failed".to_string())
}

fn is_queued(queue: &QueueResponse, prompt_id: &str) -> bool {
  queue.queue_running.iter()
    .chain(&queue.queue_pending)
    .any(|entry| entry.get(1).and_then(Value::as_str) == Some(prompt_id))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn finished_history_lists_saved_files() {
    let entry: HistoryEntry = serde_json::from_str(r#"{
      "prompt": [1, "abc", {}, {}, ["15"]],
      "outputs": {"15": {"images": [{"filename": "minimax_h3_00001_.mp4", "subfolder": "artcraft", "type": "output"}], "animated": [true]}},
      "status": {"status_str": "success", "completed": true, "messages": [["execution_start", {}]]}
    }"#).unwrap();
    assert_eq!(state_from_history(entry), PromptState::Finished(vec![FileRef {
      filename: "minimax_h3_00001_.mp4".to_string(),
      subfolder: "artcraft".to_string(),
      kind: "output".to_string(),
    }]));
  }

  #[test]
  fn failed_history_reports_the_node_error() {
    let entry: HistoryEntry = serde_json::from_str(r#"{
      "outputs": {},
      "status": {"status_str": "error", "completed": false, "messages": [
        ["execution_start", {}],
        ["execution_error", {"node_id": "11", "node_type": "SamplerCustomAdvanced", "exception_message": "CUDA out of memory.\n"}]
      ]}
    }"#).unwrap();
    assert_eq!(state_from_history(entry), PromptState::Failed("SamplerCustomAdvanced failed: CUDA out of memory.".to_string()));
  }

  #[test]
  fn queue_membership_checks_running_and_pending() {
    let queue: QueueResponse = serde_json::from_str(
      r#"{"queue_running": [[3, "running-id", {}, {}, []]], "queue_pending": [[4, "pending-id", {}, {}, []]]}"#,
    ).unwrap();
    assert!(is_queued(&queue, "running-id"));
    assert!(is_queued(&queue, "pending-id"));
    assert!(!is_queued(&queue, "other-id"));
  }
}
