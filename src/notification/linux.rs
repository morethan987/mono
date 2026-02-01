// D-Bus notification spec requires many arguments for notify()
#![allow(clippy::too_many_arguments)]

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use futures_util::StreamExt;
use tokio::sync::Mutex;
use tokio::time::timeout;
use tracing::{debug, info, warn};
use zbus::{Connection, proxy, zvariant::Value};

use crate::error::Result;
use crate::models::Task;
use crate::notification::backend::{NotificationAction, NotificationBackend};

fn notification_error(msg: impl Into<String>) -> crate::error::MonoError {
    crate::error::MonoError::Notification(msg.into())
}

#[proxy(
    interface = "org.freedesktop.Notifications",
    default_service = "org.freedesktop.Notifications",
    default_path = "/org/freedesktop/Notifications"
)]
trait Notifications {
    fn notify(
        &self,
        app_name: &str,
        replaces_id: u32,
        app_icon: &str,
        summary: &str,
        body: &str,
        actions: &[&str],
        hints: HashMap<&str, &Value<'_>>,
        expire_timeout: i32,
    ) -> zbus::Result<u32>;

    fn close_notification(&self, id: u32) -> zbus::Result<()>;

    fn get_capabilities(&self) -> zbus::Result<Vec<String>>;

    #[zbus(signal)]
    fn action_invoked(&self, id: u32, action_key: &str) -> zbus::Result<()>;

    #[zbus(signal)]
    fn notification_closed(&self, id: u32, reason: u32) -> zbus::Result<()>;
}

pub struct LinuxNotificationBackend {
    connection: Connection,
    response_timeout: Duration,
    pending_notifications: Arc<Mutex<HashMap<u32, String>>>,
}

impl LinuxNotificationBackend {
    pub async fn new() -> Result<Self> {
        let connection = Connection::session()
            .await
            .map_err(|e| notification_error(format!("无法连接 DBus session: {}", e)))?;

        Ok(Self {
            connection,
            response_timeout: Duration::from_secs(300), // 5 minutes
            pending_notifications: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.response_timeout = timeout;
        self
    }

    async fn get_proxy(&self) -> Result<NotificationsProxy<'_>> {
        NotificationsProxy::new(&self.connection)
            .await
            .map_err(|e| notification_error(format!("无法创建通知代理: {}", e)))
    }

    fn build_actions_array(actions: &[NotificationAction]) -> Vec<&str> {
        actions
            .iter()
            .flat_map(|a| [a.id.as_str(), a.label.as_str()])
            .collect()
    }
}

#[async_trait]
impl NotificationBackend for LinuxNotificationBackend {
    async fn send_task_notification(
        &self,
        task: &Task,
        actions: &[NotificationAction],
    ) -> Result<Option<String>> {
        let proxy = self.get_proxy().await?;

        let summary = "📌 壹刻提醒";
        let body = format!(
            "现在开始: {}\n预计用时: {}",
            task.title,
            task.estimated_duration_display()
        );

        let actions_array = Self::build_actions_array(actions);
        let actions_refs: Vec<&str> = actions_array.to_vec();

        let hints: HashMap<&str, &Value<'_>> = HashMap::new();

        let notification_id = proxy
            .notify(
                "mono",
                0,
                "dialog-information",
                summary,
                &body,
                &actions_refs,
                hints,
                0, // no auto-expire, wait for user action
            )
            .await
            .map_err(|e| notification_error(format!("发送通知失败: {}", e)))?;

        info!("Notification sent with id: {}", notification_id);

        let mut action_stream = proxy
            .receive_action_invoked()
            .await
            .map_err(|e| notification_error(format!("无法订阅 ActionInvoked 信号: {}", e)))?;

        let mut closed_stream = proxy
            .receive_notification_closed()
            .await
            .map_err(|e| notification_error(format!("无法订阅 NotificationClosed 信号: {}", e)))?;

        let result = timeout(self.response_timeout, async {
            let mut action_result: Option<String> = None;
            loop {
                tokio::select! {
                    // Prioritize action_stream by placing it first with biased mode
                    biased;
                    Some(signal) = action_stream.next() => {
                        if let Ok(args) = signal.args()
                            && args.id == notification_id {
                                debug!("Action invoked: id={}, action={}", args.id, args.action_key);
                                action_result = Some(args.action_key.to_string());
                                // Don't return immediately - wait for NotificationClosed to ensure cleanup
                            }
                    }
                    Some(signal) = closed_stream.next() => {
                        if let Ok(args) = signal.args()
                            && args.id == notification_id {
                                debug!("Notification closed: id={}, reason={}", args.id, args.reason);
                                // If we already received an action, return it; otherwise return None
                                return action_result;
                            }
                    }
                }
            }
        })
        .await;

        match result {
            Ok(action) => Ok(action),
            Err(_) => {
                warn!("Notification response timed out");
                let _ = proxy.close_notification(notification_id).await;
                Ok(None)
            }
        }
    }

    async fn send_simple(&self, title: &str, body: &str) -> Result<()> {
        let proxy = self.get_proxy().await?;
        let hints: HashMap<&str, &Value<'_>> = HashMap::new();
        let empty_actions: &[&str] = &[];

        proxy
            .notify(
                "mono",
                0,
                "dialog-information",
                title,
                body,
                empty_actions,
                hints,
                5000,
            )
            .await
            .map_err(|e| notification_error(format!("发送通知失败: {}", e)))?;

        Ok(())
    }

    async fn close_notification(&self, notification_id: u32) -> Result<()> {
        let proxy = self.get_proxy().await?;
        proxy
            .close_notification(notification_id)
            .await
            .map_err(|e| notification_error(format!("关闭通知失败: {}", e)))?;
        Ok(())
    }
}
