//! 数据模型：单条任务 `Task`。集合管理与持久化在 `store.rs`。

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use time::macros::format_description;

/// 单条任务。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Task {
    /// 自增 ID，删除后不回收；缺失视为文件损坏。
    pub id: u64,
    pub desc: String,
    #[serde(default)]
    pub done: bool,
    /// 本地时间 `YYYY-MM-DD HH:MM`。
    #[serde(default)]
    pub created_at: String,
}

impl Task {
    /// 新建一条未完成任务，创建时间取当前时刻。
    pub fn new(id: u64, desc: impl Into<String>) -> Self {
        Self {
            id,
            desc: desc.into(),
            done: false,
            created_at: now_local(),
        }
    }
}

/// 当前本地时间 `YYYY-MM-DD HH:MM`；取不到时区时退化为 UTC。
fn now_local() -> String {
    let now = OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc());
    now.format(format_description!("[year]-[month]-[day] [hour]:[minute]"))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_task_starts_undone_with_timestamp() {
        let task = Task::new(7, "buy milk");

        assert_eq!(task.id, 7);
        assert_eq!(task.desc, "buy milk");
        assert!(!task.done);
        assert!(!task.created_at.is_empty());
    }

    #[test]
    fn timestamp_uses_local_display_format() {
        let ts = now_local();
        let b = ts.as_bytes();

        assert_eq!(ts.len(), 16, "got: {ts}");
        assert!(
            b[4] == b'-' && b[7] == b'-' && b[10] == b' ' && b[13] == b':',
            "got: {ts}"
        );
        assert!(ts[..4].bytes().all(|c| c.is_ascii_digit()), "got: {ts}");
    }

    #[test]
    fn json_round_trip_preserves_fields() {
        let task = Task::new(1, "buy milk");

        let text = serde_json::to_string(&task).unwrap();
        let back: Task = serde_json::from_str(&text).unwrap();

        assert_eq!(back, task);
    }

    #[test]
    fn optional_fields_fall_back_to_default() {
        let back: Task = serde_json::from_str(r#"{"id":3,"desc":"x"}"#).unwrap();

        assert!(!back.done);
        assert!(back.created_at.is_empty());
    }

    #[test]
    fn required_fields_are_enforced() {
        assert!(serde_json::from_str::<Task>(r#"{"desc":"x"}"#).is_err());
        assert!(serde_json::from_str::<Task>(r#"{"id":1}"#).is_err());
    }
}
