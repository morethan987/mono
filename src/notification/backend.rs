use async_trait::async_trait;

use crate::error::Result;
use crate::models::Task;

/// 通知按钮动作
#[derive(Debug, Clone)]
pub struct NotificationAction {
    /// 动作标识符 (如 "start", "postpone_15", "skip")
    pub id: String,
    /// 显示标签 (如 "开始", "推迟15分钟", "跳过")
    pub label: String,
}

impl NotificationAction {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
        }
    }

    /// 默认任务通知动作
    pub fn default_task_actions() -> Vec<Self> {
        vec![
            Self::new("start", "开始"),
            Self::new("postpone_15", "推迟15分钟"),
            Self::new("skip", "跳过"),
        ]
    }
}

/// 平台通知后端 trait (便于后期扩展 macOS/Windows)
#[async_trait]
pub trait NotificationBackend: Send + Sync {
    /// 发送交互式任务通知，返回用户选择的 action id
    ///
    /// - 如果用户点击了某个按钮，返回 Some(action_id)
    /// - 如果用户关闭/忽略通知，返回 None
    /// - 超时时也返回 None
    async fn send_task_notification(
        &self,
        task: &Task,
        actions: &[NotificationAction],
    ) -> Result<Option<String>>;

    /// 发送简单通知 (无交互)
    async fn send_simple(&self, title: &str, body: &str) -> Result<()>;

    /// 关闭指定通知
    async fn close_notification(&self, notification_id: u32) -> Result<()>;
}
