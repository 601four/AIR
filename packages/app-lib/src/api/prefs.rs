//! AirCore 自定义偏好存储（key/value）
//!
//! 独立于巨大的 settings 表，方便随时加新开关，
//! 不需要改 settings 结构体与 SQL 占位符。

use std::collections::HashMap;

use crate::state::State;

async fn pool() -> crate::Result<sqlx::SqlitePool> {
    let state = State::get_if_initialized().ok_or_else(|| {
        crate::ErrorKind::LauncherError("state not initialized".to_string())
    })?;
    Ok(state.pool.clone())
}

/// 读取全部偏好
pub async fn prefs_get_all() -> crate::Result<HashMap<String, String>> {
    let pool = pool().await?;
    let rows: Vec<(String, String)> =
        sqlx::query_as("SELECT key, value FROM aircore_prefs")
            .fetch_all(&pool)
            .await?;
    Ok(rows.into_iter().collect())
}

/// 写入一个偏好
pub async fn prefs_set(key: String, value: String) -> crate::Result<()> {
    let pool = pool().await?;
    sqlx::query(
        "INSERT INTO aircore_prefs (key, value) VALUES (?, ?)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
    )
    .bind(key)
    .bind(value)
    .execute(&pool)
    .await?;
    Ok(())
}
