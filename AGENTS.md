# PROJECT KNOWLEDGE BASE

**Generated:** 2026-02-01
**Status:** MVP (Phase 1-3) Complete
**Branch:** main

## OVERVIEW

壹刻 (mono) - 基于 Rust 的智能任务调度引擎，采用 CLI + Daemon 架构。目前已完成核心骨架、调度引擎和 Linux 交互式通知系统。

## STRUCTURE

```
mono/
├── Cargo.toml          # 核心依赖 (zbus, sqlx, tokio, tabled)
├── src/
│   ├── main.rs         # 统一入口，处理 CLI 分发与 Daemon 启动
│   ├── cli/            # 客户端逻辑: client (IPC), display (tabled 格式化), commands (clap 定义)
│   ├── daemon/         # 守护进程: server (Unix Socket), scheduler (调度循环), state (状态管理)
│   ├── notification/   # 通知系统: backend (trait), linux (zbus/DBus), action_handler (响应处理)
│   ├── scheduling/     # 调度引擎: engine (主逻辑), policy (优先级、截止日期策略), queue (优先级队列)
│   ├── models/         # 领域模型: task, task_type, schedule, time_slot, feedback, constraints
│   ├── storage/        # 持久化层: repository (trait), sqlite (sqlx 实现)
│   ├── protocol/       # IPC 协议: request, response, codec (JSON)
│   ├── config/         # 配置管理: settings, paths (XDG)
│   ├── platform/       # 平台抽象: unix 信号处理与 PID 管理
│   └── error.rs        # 统一错误定义
├── docs/
│   └── PLANE.md        # 架构路线图 (Phases 1-6)
└── migrations/         # SQLite 数据库迁移文件
```

## IMPLEMENTATION STATUS

| 阶段 | 模块 | 状态 | 备注 |
|------|------|------|------|
| Phase 1 | 核心骨架 | ✅ 完成 | CLI, Daemon, IPC, SQLite 基础功能 |
| Phase 2 | 调度引擎 | ✅ 完成 | 基于权重、优先级、截止日期的初步调度算法 |
| Phase 3 | 交互式通知 | ✅ 完成 | Linux DBus 交互通知 + 按钮动作响应 |
| Phase 4 | 在线学习 | ⏳ 待开始 | 任务类型级 FTRL + Bandit 学习 |
| Phase 5 | 自适应整合 | ⏳ 待开始 | 反馈闭环与智能时间槽推荐 |

## WHERE TO LOOK

| 任务 | 位置 | 备注 |
|------|------|------|
| 调整调度权重 | `src/scheduling/policy/` | 优先级权重与截止日期衰减算法 |
| 修改通知 UI | `src/notification/linux.rs` | DBus Notify 参数 (summary, body) |
| 扩展通知动作 | `src/notification/action_handler.rs` | 按钮点击后的业务逻辑映射 |
| 增加 CLI 命令 | `src/cli/commands.rs` | 使用 clap derive 宏 |

## CONVENTIONS

### Error Handling
- 应用层错误使用 `crate::error::Result` (MonoError)
- 通知相关错误通过 `notification_error` 辅助函数构造

### Formatting
- 终端输出使用 `tabled` 进行美化，支持 Emoji 优先级标识
- 时间显示使用中文相对时间 (如 "3天后")

## COMMANDS

```bash
# 常用开发命令
cargo build                 # 编译
mono daemon start           # 启动守护进程 (后台)
mono daemon start -f        # 前台启动 (方便看日志)
mono add "任务标题" -p high  # 添加任务
mono now                    # 核心命令：查看现在该做什么
mono replan                 # 重新规划并查看任务评分排名
```

## NOTES

- **交互逻辑**: Daemon 启动后，Scheduler 每分钟检查一次数据库。如果存在该执行的任务，则通过 zbus 发送 DBus 通知。
- **MVP 达成**: 现在的版本已具备完整的 "添加 -> 调度 -> 提醒 -> 响应" 闭环。
