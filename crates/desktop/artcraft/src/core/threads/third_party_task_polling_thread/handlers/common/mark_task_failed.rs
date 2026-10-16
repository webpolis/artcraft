use enums::tauri::tasks::task_status::TaskStatus;
use log::{error, info};
use sqlite_tasks::queries::task::Task;
use sqlite_tasks::queries::update_task_status::{update_task_status, UpdateTaskArgs};
use tauri::AppHandle;

use crate::core::events::basic_sendable_event_trait::BasicSendableEvent;
use crate::core::events::generation_events::generation_failed_event::GenerationFailedEvent;
use crate::core::state::task_database::TaskDatabase;
use crate::core::threads::third_party_task_polling_thread::events::notify_frontend_of_completion::{
  provider_to_generation_service, task_type_to_generation_action,
};

/// Marks a third-party task failed and tells the frontend why.
pub async fn mark_task_failed(
  app_handle: &AppHandle,
  task_database: &TaskDatabase,
  task: &Task,
  reason: &str,
) {
  info!("[ThirdPartyPolling] Marking task {} as failed: {}", task.id.as_str(), reason);

  let update_result = update_task_status(UpdateTaskArgs {
    db: task_database.get_connection(),
    task_id: &task.id,
    status: TaskStatus::CompleteFailure,
  }).await;

  if let Err(err) = update_result {
    error!("[ThirdPartyPolling] Failed to update task status for {}: {:?}", task.id.as_str(), err);
  }

  GenerationFailedEvent {
    action: task_type_to_generation_action(task.task_type),
    service: provider_to_generation_service(task.provider),
    model: None,
    reason: Some(reason.to_string()),
  }.send_infallible(app_handle);
}
