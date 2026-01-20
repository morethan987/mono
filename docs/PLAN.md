🎯 壹刻 (mono)

核心理念

> "壹刻" —— 像秘书一样安排日程，让你专注于当下这一件事

---

📁 项目目录结构
```text
mono/
├── Cargo.toml
├── README.md
├── LICENSE
├── config.example.toml
│
├── migrations/
│   ├── 001_init_tasks.sql
│   ├── 002_init_schedules.sql
│   ├── 003_init_feedback.sql
│   └── 004_init_learning.sql
│
├── src/
│   ├── main.rs                      # 入口点
│   │
│   ├── cli/                         # 前台客户端
│   │   ├── mod.rs
│   │   ├── commands.rs              # 子命令定义
│   │   ├── client.rs                # IPC 客户端
│   │   └── display.rs               # 终端输出格式化
│   │
│   ├── daemon/                      # 后台守护进程
│   │   ├── mod.rs
│   │   ├── server.rs                # Unix Socket 服务
│   │   ├── scheduler.rs             # 定时调度循环
│   │   └── state.rs                 # 运行时状态管理
│   │
│   ├── notification/                # 通知系统 (交互式)
│   │   ├── mod.rs
│   │   ├── backend.rs               # 平台抽象 trait
│   │   ├── linux.rs                 # Linux DBus 实现
│   │   └── action_handler.rs        # 通知按钮响应
│   │
│   ├── models/                      # 数据模型
│   │   ├── mod.rs
│   │   ├── task.rs                  # 任务实体
│   │   ├── task_type.rs             # 任务类型 (学习用)
│   │   ├── schedule.rs              # 日程安排
│   │   ├── time_slot.rs             # 时间槽
│   │   ├── feedback.rs              # 用户反馈
│   │   └── constraints.rs           # 约束条件
│   │
│   ├── storage/                     # 持久化
│   │   ├── mod.rs
│   │   ├── repository.rs            # Repository trait
│   │   ├── sqlite.rs                # SQLite 实现
│   │   └── migrations.rs            # 迁移管理
│   │
│   ├── scheduling/                  # 调度引擎
│   │   ├── mod.rs
│   │   ├── engine.rs                # 调度引擎主逻辑
│   │   ├── policy/                  # 调度策略
│   │   │   ├── mod.rs
│   │   │   ├── trait.rs             # SchedulingPolicy trait
│   │   │   ├── priority.rs          # 优先级调度
│   │   │   ├── deadline.rs          # 截止日期优先
│   │   │   └── adaptive.rs          # 自适应调度
│   │   ├── constraints/             # 约束求解
│   │   │   ├── mod.rs
│   │   │   ├── validator.rs
│   │   │   └── solver.rs
│   │   └── queue.rs                 # 优先级队列
│   │
│   ├── learning/                    # 在线学习 (任务类型级)
│   │   ├── mod.rs
│   │   ├── task_type_model.rs       # 每类型独立模型
│   │   ├── features.rs              # 特征工程
│   │   ├── ftrl.rs                  # FTRL 增量学习
│   │   ├── bandit.rs                # Multi-armed Bandit
│   │   └── reward.rs                # Reward 计算
│   │
│   ├── protocol/                    # IPC 协议
│   │   ├── mod.rs
│   │   ├── request.rs               # 请求消息
│   │   ├── response.rs              # 响应消息
│   │   └── codec.rs                 # 序列化
│   │
│   ├── config/                      # 配置管理
│   │   ├── mod.rs
│   │   ├── settings.rs              # 配置结构
│   │   └── paths.rs                 # XDG 路径
│   │
│   └── platform/                    # 平台抽象层 (便于后期扩展)
│       ├── mod.rs
│       ├── traits.rs                # 平台相关 trait
│       └── unix.rs                  # Unix 实现
│
└── tests/
    ├── integration/
    └── fixtures/
```

---

🛠 技术栈

```[package]
name = "mono"
version = "0.1.0"
edition = "2024"
authors = "morethan"
description = "壹刻 - 智能日程规划引擎，让你专注当下"

[dependencies]
anyhow = "1.0.100"
async-trait = "0.1.89"
chrono = { version = "0.4.43", features = ["serde"] }
clap = { version = "4.5.54", features = ["derive", "env"] }
daemonize2 = "0.6.2"
directories = "6.0.0"
ndarray = "0.17.2"
owo-colors = "4.2.3"
serde = { version = "1.0.228", features = ["derive"] }
serde_json = "1.0.149"
sqlx = { version = "0.8.6", features = ["sqlite", "runtime-tokio", "chrono", "uuid"] }
thiserror = "2.0.18"
tokio = { version = "1.49.0", features = ["full"] }
toml = "0.9.11"
tracing = "0.1.44"
tracing-subscriber = { version = "0.3.22", features = ["env-filter"] }
uuid = "1.19.0"
zbus = "5.13.2"

[dev-dependencies]
tempfile = "3.24.0"
tokio-test = "0.4.5"
```

> 注意: 交互式通知在 Linux 上需要使用 zbus 直接操作 DBus，而不是 notify-rust（后者对 action 回调支持有限）

---

📋 CLI 命令设计
```text
mono - 壹刻，你的智能日程秘书
USAGE:
    mono <COMMAND>
COMMANDS:
    daemon      守护进程管理
    add         添加新任务
    list        列出任务
    now         查看当前应该做什么 ⭐ (核心命令)
    today       查看今日日程
    complete    标记任务完成
    postpone    推迟任务
    feedback    提交详细反馈
    replan      重新规划日程
    config      配置管理
    help        帮助信息
EXAMPLES:
    mono daemon start           启动守护进程
    mono add "完成项目报告" -p high -d tomorrow
    mono now                    当前该做什么？
    mono complete abc123        标记完成
    mono feedback abc123        提交详细反馈
核心交互流程
用户启动 mono → daemon 在后台运行
           ↓
    daemon 根据算法规划日程
           ↓
    到达任务时间 → 推送交互式通知
           ↓
    ┌──────────────────────────────────┐
    │  📌 壹刻提醒                     │
    │  现在开始: 完成项目报告          │
    │  预计用时: 45 分钟               │
    │                                  │
    │  [开始] [推迟15分钟] [跳过]      │
    └──────────────────────────────────┘
           ↓
    用户点击按钮 → 实时反馈给 daemon
           ↓
    学习模块更新该任务类型的模型
```

---

🔔 交互式通知设计 (Linux DBus)
```rust
// src/notification/backend.rs
use async_trait::async_trait;
/// 平台通知后端 trait (便于后期扩展 macOS/Windows)
#[async_trait]
pub trait NotificationBackend: Send + Sync {
    /// 发送交互式通知，返回用户选择的 action
    async fn send_task_notification(
        &self,
        task: &Task,
        actions: &[NotificationAction],
    ) -> Result<Option<String>>;  // 返回用户点击的 action id
    
    /// 发送简单通知 (无交互)
    async fn send_simple(&self, title: &str, body: &str) -> Result<()>;
}
#[derive(Debug, Clone)]
pub struct NotificationAction {
    pub id: String,      // "start", "postpone_15", "skip"
    pub label: String,   // "开始", "推迟15分钟", "跳过"
}
// src/notification/linux.rs
use zbus::{Connection, dbus_proxy};
pub struct LinuxNotificationBackend {
    connection: Connection,
}
impl LinuxNotificationBackend {
    pub async fn new() -> Result<Self> {
        let connection = Connection::session().await?;
        Ok(Self { connection })
    }
}
#[async_trait]
impl NotificationBackend for LinuxNotificationBackend {
    async fn send_task_notification(
        &self,
        task: &Task,
        actions: &[NotificationAction],
    ) -> Result<Option<String>> {
        // 构建 actions 数组: ["start", "开始", "postpone_15", "推迟15分钟", ...]
        let action_list: Vec<&str> = actions
            .iter()
            .flat_map(|a| [a.id.as_str(), a.label.as_str()])
            .collect();
        
        // 调用 org.freedesktop.Notifications.Notify
        let notification_id = self.send_notification(
            "壹刻提醒",
            &format!("现在开始: {}\n预计用时: {}", task.title, task.estimated_duration_display()),
            &action_list,
            0,  // 不自动消失
        ).await?;
        
        // 监听 ActionInvoked 信号
        let action = self.wait_for_action(notification_id).await?;
        Ok(action)
    }
}
```

---

📊 任务类型级在线学习
```rust
// src/models/task_type.rs
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct TaskType {
    pub name: String,  // "工作", "学习", "运动", "家务", "社交" 等
}
impl TaskType {
    pub fn from_tags(tags: &[String]) -> Self {
        // 从 tags 推断任务类型，或使用默认
        tags.first()
            .map(|t| TaskType { name: t.clone() })
            .unwrap_or_else(|| TaskType { name: "默认".to_string() })
    }
}
// src/learning/task_type_model.rs
use std::collections::HashMap;
/// 每个任务类型有独立的学习模型
pub struct TaskTypeLearningModel {
    task_type: TaskType,
    
    // 时间槽偏好 Bandit (探索哪个时间段最适合这类任务)
    time_slot_bandit: TimeSlotBandit,
    
    // FTRL 模型 (预测成功概率)
    ftrl_model: FtrlModel,
    
    // 统计数据
    total_scheduled: u32,
    total_completed: u32,
    avg_completion_rate: f64,
    best_time_slots: Vec<TimeSlotStats>,
}
/// 管理所有任务类型的学习模型
pub struct LearningManager {
    models: HashMap<TaskType, TaskTypeLearningModel>,
    global_model: GlobalLearningModel,  // 跨类型的通用模式
}
impl LearningManager {
    /// 获取或创建某任务类型的模型
    pub fn get_model(&mut self, task_type: &TaskType) -> &mut TaskTypeLearningModel {
        self.models
            .entry(task_type.clone())
            .or_insert_with(|| TaskTypeLearningModel::new(task_type.clone()))
    }
    
    /// 预测任务在某时间槽的成功概率
    pub fn predict_success(
        &self,
        task: &Task,
        time_slot: &TimeSlot,
        context: &Context,
    ) -> f64 {
        let task_type = TaskType::from_tags(&task.tags);
        
        if let Some(model) = self.models.get(&task_type) {
            // 有历史数据，使用类型专属模型
            let type_prediction = model.predict(task, time_slot, context);
            let global_prediction = self.global_model.predict(task, time_slot, context);
            
            // 加权组合 (随数据增多，更信任专属模型)
            let weight = (model.total_scheduled as f64 / 20.0).min(1.0);
            weight * type_prediction + (1.0 - weight) * global_prediction
        } else {
            // 冷启动，使用全局模型
            self.global_model.predict(task, time_slot, context)
        }
    }
    
    /// 根据反馈更新模型
    pub fn update_from_feedback(&mut self, task: &Task, feedback: &Feedback) {
        let task_type = TaskType::from_tags(&task.tags);
        let reward = compute_reward(feedback);
        
        // 更新类型专属模型
        let model = self.get_model(&task_type);
        model.update(task, feedback, reward);
        
        // 同时更新全局模型
        self.global_model.update(task, feedback, reward);
    }
}
```

---

🗄 数据存储 (XDG 标准)
```rust
// src/config/paths.rs
use directories::ProjectDirs;
use std::path::PathBuf;
pub struct MonoPaths {
    /// ~/.local/share/mono/
    pub data_dir: PathBuf,
    /// ~/.config/mono/
    pub config_dir: PathBuf,
    /// ~/.local/share/mono/mono.db
    pub database: PathBuf,
    /// /run/user/{uid}/mono.sock
    pub socket: PathBuf,
    /// ~/.local/share/mono/mono.pid
    pub pid_file: PathBuf,
}
impl MonoPaths {
    pub fn new() -> Result<Self> {
        let proj_dirs = ProjectDirs::from("", "", "mono")
            .ok_or_else(|| anyhow!("无法确定数据目录"))?;
        
        let data_dir = proj_dirs.data_dir().to_path_buf();
        let config_dir = proj_dirs.config_dir().to_path_buf();
        
        // Socket 放在 runtime dir (如果可用) 或 data dir
        let socket = std::env::var("XDG_RUNTIME_DIR")
            .map(|dir| PathBuf::from(dir).join("mono.sock"))
            .unwrap_or_else(|_| data_dir.join("mono.sock"));
        
        Ok(Self {
            database: data_dir.join("mono.db"),
            pid_file: data_dir.join("mono.pid"),
            socket,
            data_dir,
            config_dir,
        })
    }
}
```

---

📅 实现路线图

| 阶段 | 周期 | 目标 | 交付物 |
|------|------|------|--------|
| Phase 1 | Week 1-2 | 核心骨架 | CLI + Daemon + IPC + SQLite |
| Phase 2 | Week 3-4 | 基础调度 | 优先级/截止日期调度 + mono now |
| Phase 3 | Week 5 | 交互式通知 | Linux DBus 通知 + 按钮响应 |
| Phase 4 | Week 6-7 | 在线学习 | 任务类型级 Bandit + FTRL |
| Phase 5 | Week 8 | 自适应整合 | 智能调度 + 反馈闭环 |
| Phase 6 | Week 9-10 | 打磨发布 | 测试、文档、优化 |

---

🎯 MVP 定义 (最小可行产品)

Phase 1-3 完成后即可使用：
1. ✅ mono daemon start/stop - 守护进程管理
2. ✅ mono add - 添加任务
3. ✅ mono list / mono today - 查看任务
4. ✅ mono now - 当前该做什么
5. ✅ 交互式通知 (开始/推迟/跳过)
6. ✅ 基于优先级 + 截止日期的简单调度
Phase 4-5 完成后具备智能：
7. ✅ 任务类型级学习
8. ✅ 自适应时间槽推荐
9. ✅ 详细反馈收集 (mono feedback)
