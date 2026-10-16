//! Starts the local ComfyUI server when a local model is used and nothing answers, and stops it
//! again when ArtCraft exits. A server the user started themselves is never touched.

use std::fs::File;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use comfyui_client::server::is_running;
use log::{info, warn};

use crate::core::commands::generate::comfyui::COMFYUI_HOST;

/// Loading the custom nodes takes 20–40 seconds; leave room for a cold disk cache.
const STARTUP_TIMEOUT: Duration = Duration::from_secs(180);
const STARTUP_POLL: Duration = Duration::from_secs(1);

/// The server ArtCraft started, if any.
static STARTED_SERVER: Mutex<Option<Child>> = Mutex::new(None);
/// Makes a second request wait for the first launch instead of starting another server.
static LAUNCH_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Returns once ComfyUI answers, starting it with `launcher` if needed. The error is a message for
/// the user.
pub async fn ensure_comfyui_running(launcher: Option<&Path>, log_path: &Path) -> Result<(), String> {
  if is_running(COMFYUI_HOST).await {
    return Ok(());
  }
  let _launch = LAUNCH_LOCK.lock().await;
  if is_running(COMFYUI_HOST).await {
    return Ok(());
  }
  let launcher = launcher.ok_or_else(|| format!(
    "ComfyUI isn't running at {}. Start it, or set \"comfyui_launcher\" in ~/Artcraft/settings/env_configs.json so ArtCraft can start it.",
    COMFYUI_HOST,
  ))?;

  if !server_still_starting() {
    start_server(launcher, log_path)?;
  }
  wait_until_ready(log_path).await
}

/// Called when ArtCraft exits.
pub fn stop_started_comfyui() {
  let Some(mut child) = STARTED_SERVER.lock().expect("lock is never poisoned").take() else {
    return;
  };
  info!("Stopping the ComfyUI server ArtCraft started (pid {})", child.id());
  if let Err(err) = child.kill().and_then(|_| child.wait().map(|_| ())) {
    warn!("Could not stop ComfyUI: {}", err);
  }
}

/// True when an earlier launch is still booting (it timed out before ComfyUI answered).
fn server_still_starting() -> bool {
  let mut started = STARTED_SERVER.lock().expect("lock is never poisoned");
  matches!(started.as_mut().map(Child::try_wait), Some(Ok(None)))
}

fn start_server(launcher: &Path, log_path: &Path) -> Result<(), String> {
  info!("Starting ComfyUI with {:?}; log at {:?}", launcher, log_path);
  let log = File::create(log_path).map_err(|err| format!("Could not create {}: {}", log_path.display(), err))?;
  let log_copy = log.try_clone().map_err(|err| format!("Could not open {}: {}", log_path.display(), err))?;
  let child = Command::new(launcher)
    .current_dir(launcher.parent().unwrap_or(Path::new("/")))
    .stdin(Stdio::null())
    .stdout(log)
    .stderr(log_copy)
    .spawn()
    .map_err(|err| format!("Could not start ComfyUI with {}: {}", launcher.display(), err))?;
  *STARTED_SERVER.lock().expect("lock is never poisoned") = Some(child);
  Ok(())
}

async fn wait_until_ready(log_path: &Path) -> Result<(), String> {
  let deadline = Instant::now() + STARTUP_TIMEOUT;
  while Instant::now() < deadline {
    if is_running(COMFYUI_HOST).await {
      info!("ComfyUI is ready");
      return Ok(());
    }
    if let Some(status) = exited_status() {
      return Err(format!("ComfyUI stopped while starting ({}). Its log: {}", status, log_path.display()));
    }
    tokio::time::sleep(STARTUP_POLL).await;
  }
  Err(format!("ComfyUI didn't start within {} seconds. Its log: {}", STARTUP_TIMEOUT.as_secs(), log_path.display()))
}

fn exited_status() -> Option<String> {
  let mut started = STARTED_SERVER.lock().expect("lock is never poisoned");
  match started.as_mut().map(Child::try_wait) {
    Some(Ok(Some(status))) => {
      started.take();
      Some(status.to_string())
    },
    _ => None,
  }
}
