use std::time::Duration;

use tokio::time;

use crate::{AppState, auth, middleware};

const CLEANUP_INTERVAL: Duration = Duration::from_secs(60 * 60);

/// 启动内存状态定时清理任务。
pub fn spawn_cleanup_task(state: AppState) {
    tokio::spawn(async move {
        let mut interval = time::interval(CLEANUP_INTERVAL);

        loop {
            interval.tick().await;
            let removed_sessions = auth::cleanup_expired_sessions(&state).await;
            let removed_login_limits = middleware::cleanup_expired_login_rate_limits(&state).await;
            let removed_duplicate_submissions =
                middleware::cleanup_expired_duplicate_submissions(&state).await;

            if removed_sessions > 0 || removed_login_limits > 0 || removed_duplicate_submissions > 0
            {
                tracing::debug!(
                    removed_sessions,
                    removed_login_limits,
                    removed_duplicate_submissions,
                    "cleaned expired in-memory records"
                );
            }
        }
    });
}
