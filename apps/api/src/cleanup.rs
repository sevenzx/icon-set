use std::time::Duration;

use tokio::time;

use crate::{AppState, middleware};

const CLEANUP_INTERVAL: Duration = Duration::from_secs(60 * 60);

/// 启动定时清理任务。
pub fn spawn_cleanup_task(state: AppState) {
    tokio::spawn(async move {
        let mut interval = time::interval(CLEANUP_INTERVAL);

        loop {
            interval.tick().await;
            let removed_database_records = match state.db.cleanup_expired().await {
                Ok(count) => count,
                Err(err) => {
                    tracing::warn!(error = %err, "failed to clean expired database records");
                    0
                }
            };
            let removed_duplicate_submissions =
                middleware::cleanup_expired_duplicate_submissions(&state).await;

            if removed_database_records > 0 || removed_duplicate_submissions > 0 {
                tracing::debug!(
                    removed_database_records,
                    removed_duplicate_submissions,
                    "cleaned expired runtime records"
                );
            }
        }
    });
}
