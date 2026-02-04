# PROJECT KNOWLEDGE BASE

**Generated:** 2026-02-01
**Status:** Phase 5 Complete (Learning + Persistence + Feedback)
**Branch:** main

## OVERVIEW

壹刻 (mono) - 基于 Rust 的智能任务调度引擎，采用 CLI + Daemon 架构。已完成在线学习系统（FTRL + Thompson Sampling），支持学习模型持久化和交互式反馈收集。

## STRUCTURE

```
mono/
├── Cargo.toml          # 核心依赖 (zbus, sqlx, tokio, tabled, atty)
├── src/
│   ├── main.rs         # 统一入口，处理 CLI 分发与 Daemon 启动
│   ├── cli/            # 客户端逻辑: client (IPC), display (tabled 格式化), commands (clap 定义)
│   ├── daemon/         # 守护进程: server (Unix Socket), scheduler (调度循环), state (状态管理 + 学习模型)
│   ├── notification/   # 通知系统: backend (trait), linux (zbus/DBus), action_handler (响应处理)
│   ├── scheduling/     # 调度引擎: engine (主逻辑), policy (优先级、截止日期、学习策略), queue (优先级队列)
│   ├── learning/       # 在线学习: manager (核心), ftrl (增量学习), bandit (时段优化), features (特征工程), reward (奖励计算)
│   ├── models/         # 领域模型: task, task_type, schedule, time_slot, feedback, constraints
│   ├── storage/        # 持久化层: repository (trait), sqlite (sqlx 实现, 含学习模型存储)
│   ├── protocol/       # IPC 协议: request, response, codec (JSON)
│   ├── config/         # 配置管理: settings, paths (XDG)
│   ├── platform/       # 平台抽象: unix 信号处理与 PID 管理
│   └── error.rs        # 统一错误定义
├── docs/
│   └── PLAN.md         # 架构路线图 (Phases 1-6)
└── migrations/         # SQLite 数据库迁移文件
    ├── 001_init_tasks.sql
    ├── 002_init_schedules.sql
    ├── 003_init_feedback.sql
    ├── 004_init_learning.sql
    └── 005_learning_models.sql  # 学习模型持久化表
```

模块声明采用最新的模块名方式，不采用 mod.rs 文件的方式，对齐最新 Rust 标准

## IMPLEMENTATION STATUS

| 阶段 | 模块 | 状态 | 备注 |
|------|------|------|------|
| Phase 1 | 核心骨架 | ✅ 完成 | CLI, Daemon, IPC, SQLite 基础功能 |
| Phase 2 | 调度引擎 | ✅ 完成 | 基于权重、优先级、截止日期的初步调度算法 |
| Phase 3 | 交互式通知 | ✅ 完成 | Linux DBus 交互通知 + 按钮动作响应 |
| Phase 4 | 在线学习 | ✅ 完成 | FTRL 模型 + Thompson Sampling Bandit |
| Phase 5 | 自适应整合 | ✅ 完成 | 反馈闭环、模型持久化、交互式反馈、时段推荐 |
| Phase 7-10 | 秘书化演进 | ✅ 完成 | 任务关联、衍生任务、智能推断、行为学习 |
| Phase 11 | 环境感知 | ✅ 完成 | Niri IPC 接入、应用分类、标题分析 |
| Phase 12 | 行为自动驾驶 | ✅ 完成 | Bayesian 时长预测、自动中断检测、时间衰减 |
| Phase 6 | Web UI | ⏳ 待开始 | Web 控制面板 |

## LEARNING SYSTEM ARCHITECTURE

```
LearningManager
├── GlobalLearningModel          # 跨类型通用模式
│   ├── ftrl_model              # FTRL 增量学习
│   ├── time_slot_bandit        # Thompson Sampling 时段优化
│   └── total_tasks
└── HashMap<String, TaskTypeLearningModel>  # 每任务类型独立模型
    ├── task_type
    ├── time_slot_bandit        # 类型专属时段偏好
    ├── ftrl_model              # 类型专属成功预测
    ├── total_scheduled/completed/postponed/skipped
    └── avg_duration_minutes
```

**置信度混合**: 使用 sqrt 缩放实现冷启动处理
- 0 样本 → 100% 全局模型
- 2.5 样本 → 50/50 混合
- 10+ 样本 → 100% 类型专属模型

## WHERE TO LOOK

| 任务 | 位置 | 备注 |
|------|------|------|
| 调整调度权重 | `src/scheduling/policy/` | 优先级权重与截止日期衰减算法 |
| 修改通知 UI | `src/notification/linux.rs` | DBus Notify 参数 (summary, body) |
| 扩展通知动作 | `src/notification/action_handler.rs` | 按钮点击后的业务逻辑映射 |
| 增加 CLI 命令 | `src/cli/commands.rs` | 使用 clap derive 宏 |
| 调整学习算法 | `src/learning/manager.rs` | predict_success(), update_from_feedback() |
| FTRL 超参数 | `src/learning/ftrl.rs` | alpha, beta, lambda1, lambda2 |
| 时段 Bandit | `src/learning/bandit.rs` | Thompson Sampling 实现 |
| 特征工程 | `src/learning/features.rs` | 任务特征提取 |
| 模型持久化 | `src/storage/sqlite.rs` | load/save_learning_manager() |
| 学习统计显示 | `src/main.rs` handle_stats() | mono stats 命令 |

## CONVENTIONS

### Error Handling
- 应用层错误使用 `crate::error::Result` (MonoError)
- 通知相关错误通过 `notification_error` 辅助函数构造
- JSON 序列化错误自动转换为 `MonoError::JsonSerialization`

### Formatting
- 终端输出使用 `tabled` 进行美化，支持 Emoji 优先级标识
- 时间显示使用中文相对时间 (如 "3天后")
- 进度条使用 Unicode 字符 (█░)

### Learning Model Persistence
- 模型在 DaemonState 初始化时从数据库加载
- 每 10 次学习更新自动保存
- Daemon 关闭时保存最终状态

## COMMANDS

```bash
# 常用开发命令
cargo build                 # 编译
cargo test                  # 运行测试 (56 tests)
mono daemon start           # 启动守护进程 (后台)
mono daemon start -f        # 前台启动 (方便看日志)
mono add "任务标题" -p high  # 添加任务
mono now                    # 核心命令：查看现在该做什么 + 时段推荐
mono replan                 # 重新规划并查看任务评分排名
mono complete <id>          # 完成任务 (支持交互式反馈)
mono complete <id> -s       # 完成任务 (跳过反馈)
mono stats                  # 查看学习统计
mono stats -v               # 详细统计 (含时段成功率)
mono feedback <id> -r 5 -d 3 -e 4  # 提交详细反馈
```

## NOTES

- **交互逻辑**: Daemon 启动后，Scheduler 每分钟检查一次数据库。如果存在该执行的任务，则通过 zbus 发送 DBus 通知。
- **学习闭环**: 完成/推迟任务时自动更新学习模型 → 模型影响下次调度排序
- **时段推荐**: `mono now` 显示任务时会附带推荐执行时段（置信度 > 30% 时显示）
- **交互式反馈**: 终端检测 (atty) 确保只在交互模式下提示反馈
