// Allow dead code during development - many features are staged for future use
#![allow(dead_code)]

mod cli;
mod config;
mod daemon;
mod error;
mod learning;
mod models;
mod notification;
mod platform;
mod protocol;
mod scheduling;
mod storage;

use clap::Parser;
use tracing_subscriber::EnvFilter;

use crate::cli::{
    Cli, Commands, ConfigAction, DaemonAction, DaemonClient, StatsAction, format_ranked_task_list,
    format_task_detail, format_task_list, parse_deadline, print_error, print_info, print_success,
    print_warning,
};
use crate::config::{MonoPaths, Settings};
use crate::daemon::{daemon_status, run_daemon_background, run_daemon_foreground, stop_daemon};
use crate::models::TaskStatus;
use crate::protocol::{Request, Response, TimeSlotDetail};
use owo_colors::OwoColorize;
use tabled::{Table, Tabled, settings::Style};

fn main() {
    let cli = Cli::parse();

    if let Commands::Daemon {
        action: DaemonAction::Start { foreground: false },
    } = &cli.command
    {
        let paths = match MonoPaths::new() {
            Ok(p) => p,
            Err(e) => {
                print_error(&e.to_string());
                std::process::exit(1);
            }
        };

        if let Err(e) = paths.ensure_dirs() {
            print_error(&e.to_string());
            std::process::exit(1);
        }

        print_info("正在启动守护进程...");

        if let Err(e) = run_daemon_background(&paths) {
            print_error(&e.to_string());
            std::process::exit(1);
        }
        return;
    }

    tokio_main(cli);
}

#[tokio::main]
async fn tokio_main(cli: Cli) {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn")),
        )
        .init();

    if let Err(e) = run(cli).await {
        print_error(&e.to_string());
        std::process::exit(1);
    }
}

async fn run(cli: Cli) -> error::Result<()> {
    let paths = MonoPaths::new()?;

    match cli.command {
        Commands::Daemon { action } => handle_daemon(action, &paths).await,
        Commands::Add(args) => handle_add(args, &paths).await,
        Commands::List(args) => handle_list(args, &paths).await,
        Commands::Now => handle_now(&paths).await,
        Commands::Today => handle_today(&paths).await,
        Commands::Complete(args) => handle_complete(args, &paths).await,
        Commands::Postpone(args) => handle_postpone(args, &paths).await,
        Commands::Delete(args) => handle_delete(args, &paths).await,
        Commands::Update(args) => handle_update(args, &paths).await,
        Commands::Feedback(args) => handle_feedback(args, &paths).await,
        Commands::Replan => handle_replan(&paths).await,
        Commands::Stats { action } => handle_stats(action, &paths).await,
        Commands::Config { action } => handle_config(action, &paths),
    }
}

async fn handle_daemon(action: DaemonAction, paths: &MonoPaths) -> error::Result<()> {
    match action {
        DaemonAction::Start { foreground } => {
            if foreground {
                paths.ensure_dirs()?;
                print_info("在前台模式启动守护进程...");
                run_daemon_foreground(paths).await?;
            }
            Ok(())
        }
        DaemonAction::Stop => match stop_daemon(paths) {
            Ok(()) => {
                print_success("守护进程已停止");
                Ok(())
            }
            Err(e) => Err(e),
        },
        DaemonAction::Status => {
            match daemon_status(paths)? {
                Some(pid) => {
                    print_success(&format!("守护进程正在运行 (PID: {})", pid));

                    let settings = Settings::load(&paths.config_file()).unwrap_or_default();
                    if let Ok(mut client) =
                        DaemonClient::connect(&paths.socket, settings.daemon.ipc_timeout_secs).await
                        && let Ok(Response::DaemonStatus {
                            uptime_secs,
                            task_count,
                            ..
                        }) = client.request(Request::GetDaemonStatus).await
                    {
                        println!("  运行时间: {}s", uptime_secs);
                        println!("  任务数量: {}", task_count);
                    }
                }
                None => {
                    print_warning("守护进程未运行");
                }
            }
            Ok(())
        }
    }
}

async fn handle_add(args: cli::AddArgs, paths: &MonoPaths) -> error::Result<()> {
    let settings = Settings::load(&paths.config_file()).unwrap_or_default();
    let mut client = DaemonClient::connect(&paths.socket, settings.daemon.ipc_timeout_secs).await?;

    let deadline = args.deadline.as_ref().and_then(|d| parse_deadline(d));

    let response = client
        .request(Request::AddTask {
            title: args.title,
            description: args.description,
            priority: args.priority.map(|p| p.into()),
            tags: args.tag,
            estimated_minutes: args.estimated,
            deadline,
        })
        .await?;

    match response {
        Response::Task { task } => {
            print_success(&format!("任务已创建: {}", task.short_id()));
            println!("{}", format_task_detail(&task));
        }
        Response::Error { message } => {
            print_error(&message);
        }
        _ => {
            print_error("意外的响应");
        }
    }

    Ok(())
}

async fn handle_list(args: cli::ListArgs, paths: &MonoPaths) -> error::Result<()> {
    let settings = Settings::load(&paths.config_file()).unwrap_or_default();
    let mut client = DaemonClient::connect(&paths.socket, settings.daemon.ipc_timeout_secs).await?;

    let status = if args.all {
        None
    } else {
        args.status.as_ref().map(|s| TaskStatus::from_str(s))
    };

    let response = client
        .request(Request::ListTasks {
            status,
            limit: args.limit,
        })
        .await?;

    match response {
        Response::TaskList { tasks } => {
            println!("{}", format_task_list(&tasks));
        }
        Response::Error { message } => {
            print_error(&message);
        }
        _ => {
            print_error("意外的响应");
        }
    }

    Ok(())
}

async fn handle_now(paths: &MonoPaths) -> error::Result<()> {
    let settings = Settings::load(&paths.config_file()).unwrap_or_default();
    let mut client = DaemonClient::connect(&paths.socket, settings.daemon.ipc_timeout_secs).await?;

    let response = client.request(Request::GetCurrentTask).await?;

    match response {
        Response::CurrentTask { task: Some(task) } => {
            println!("\n📌 现在应该做:\n");
            println!("{}", format_task_detail(&task));

            let rec_response = client
                .request(Request::GetTimeSlotRecommendation {
                    task_id: task.short_id().to_string(),
                })
                .await?;
            if let Response::TimeSlotRecommendation {
                recommended_slot,
                confidence,
                ..
            } = rec_response
                && confidence > 0.3
            {
                println!(
                    "\n💡 推荐时段: {} (置信度: {:.0}%)",
                    recommended_slot,
                    confidence * 100.0
                );
            }
        }
        Response::CurrentTask { task: None } => {
            print_info("没有待办任务，享受当下吧！🎉");
        }
        Response::Error { message } => {
            print_error(&message);
        }
        _ => {
            print_error("意外的响应");
        }
    }

    Ok(())
}

async fn handle_today(paths: &MonoPaths) -> error::Result<()> {
    let settings = Settings::load(&paths.config_file()).unwrap_or_default();
    let mut client = DaemonClient::connect(&paths.socket, settings.daemon.ipc_timeout_secs).await?;

    let response = client.request(Request::ListToday).await?;

    match response {
        Response::TaskList { tasks } => {
            if tasks.is_empty() {
                print_info("今日没有安排的任务");
            } else {
                println!("\n📅 今日任务 ({} 项):\n", tasks.len());
                println!("{}", format_task_list(&tasks));
            }
        }
        Response::Error { message } => {
            print_error(&message);
        }
        _ => {
            print_error("意外的响应");
        }
    }

    Ok(())
}

async fn handle_replan(paths: &MonoPaths) -> error::Result<()> {
    let settings = Settings::load(&paths.config_file()).unwrap_or_default();
    let mut client = DaemonClient::connect(&paths.socket, settings.daemon.ipc_timeout_secs).await?;

    let response = client.request(Request::Replan).await?;

    match response {
        Response::RankedTasks { tasks } => {
            if tasks.is_empty() {
                print_info("没有待办任务需要规划");
            } else {
                println!("\n🔄 重新规划完成 ({} 项任务):\n", tasks.len());
                println!("{}", format_ranked_task_list(&tasks));
                println!();
                if let Some(first) = tasks.first() {
                    print_success(&format!("下一个任务: {}", first.task.title));
                }
            }
        }
        Response::Error { message } => {
            print_error(&message);
        }
        _ => {
            print_error("意外的响应");
        }
    }

    Ok(())
}

async fn handle_complete(args: cli::CompleteArgs, paths: &MonoPaths) -> error::Result<()> {
    let settings = Settings::load(&paths.config_file()).unwrap_or_default();
    let mut client = DaemonClient::connect(&paths.socket, settings.daemon.ipc_timeout_secs).await?;

    let response = client
        .request(Request::CompleteTask {
            id: args.id.clone(),
            actual_minutes: args.time,
        })
        .await?;

    match response {
        Response::Task { task } => {
            print_success(&format!("任务已完成: {}", task.title));

            if !args.skip_feedback && atty::is(atty::Stream::Stdin) {
                println!();
                if let Some(feedback) = prompt_for_feedback() {
                    let feedback_response = client
                        .request(Request::SubmitFeedback {
                            task_id: args.id.clone(),
                            rating: feedback.rating,
                            difficulty: feedback.difficulty,
                            energy_level: feedback.energy,
                            notes: None,
                        })
                        .await?;

                    if let Response::Ok = feedback_response {
                        print_info("感谢反馈！这将帮助优化未来的任务调度");
                    }
                }
            }
        }
        Response::Error { message } => {
            print_error(&message);
        }
        _ => {
            print_error("意外的响应");
        }
    }

    Ok(())
}

struct QuickFeedback {
    rating: Option<u8>,
    difficulty: Option<u8>,
    energy: Option<u8>,
}

fn prompt_for_feedback() -> Option<QuickFeedback> {
    use std::io::{BufRead, Write};

    print!("快速反馈 (1-5分，回车跳过): 满意度? ");
    std::io::stdout().flush().ok();

    let stdin = std::io::stdin();
    let mut lines = stdin.lock().lines();

    let rating = lines
        .next()
        .and_then(|r| r.ok())
        .and_then(|s| s.trim().parse::<u8>().ok())
        .filter(|&r| (1..=5).contains(&r));

    rating?;

    print!("难度 (1=容易, 5=困难)? ");
    std::io::stdout().flush().ok();
    let difficulty = lines
        .next()
        .and_then(|r| r.ok())
        .and_then(|s| s.trim().parse::<u8>().ok())
        .filter(|&d| (1..=5).contains(&d));

    print!("当前精力 (1=疲惫, 5=充沛)? ");
    std::io::stdout().flush().ok();
    let energy = lines
        .next()
        .and_then(|r| r.ok())
        .and_then(|s| s.trim().parse::<u8>().ok())
        .filter(|&e| (1..=5).contains(&e));

    Some(QuickFeedback {
        rating,
        difficulty,
        energy,
    })
}

async fn handle_postpone(args: cli::PostponeArgs, paths: &MonoPaths) -> error::Result<()> {
    let settings = Settings::load(&paths.config_file()).unwrap_or_default();
    let mut client = DaemonClient::connect(&paths.socket, settings.daemon.ipc_timeout_secs).await?;

    let response = client
        .request(Request::PostponeTask {
            id: args.id.clone(),
            minutes: args.minutes,
        })
        .await?;

    match response {
        Response::Task { task } => {
            print_success(&format!("任务已推迟 {} 分钟: {}", args.minutes, task.title));
        }
        Response::Error { message } => {
            print_error(&message);
        }
        _ => {
            print_error("意外的响应");
        }
    }

    Ok(())
}

async fn handle_delete(args: cli::DeleteArgs, paths: &MonoPaths) -> error::Result<()> {
    let settings = Settings::load(&paths.config_file()).unwrap_or_default();
    let mut client = DaemonClient::connect(&paths.socket, settings.daemon.ipc_timeout_secs).await?;

    if !args.force {
        let get_response = client
            .request(Request::GetTask {
                id: args.id.clone(),
            })
            .await?;

        match get_response {
            Response::Task { task } => {
                println!("即将删除任务:");
                println!("{}", format_task_detail(&task));
                print!("确认删除? [y/N] ");
                use std::io::Write;
                std::io::stdout().flush().ok();

                let mut input = String::new();
                std::io::stdin().read_line(&mut input).ok();

                if !input.trim().eq_ignore_ascii_case("y") {
                    print_info("已取消删除");
                    return Ok(());
                }
            }
            Response::Error { message } => {
                print_error(&message);
                return Ok(());
            }
            _ => {
                print_error("意外的响应");
                return Ok(());
            }
        }
    }

    let response = client
        .request(Request::DeleteTask {
            id: args.id.clone(),
        })
        .await?;

    match response {
        Response::Ok => {
            print_success("任务已删除");
        }
        Response::Error { message } => {
            print_error(&message);
        }
        _ => {
            print_error("意外的响应");
        }
    }

    Ok(())
}

async fn handle_update(args: cli::UpdateArgs, paths: &MonoPaths) -> error::Result<()> {
    let settings = Settings::load(&paths.config_file()).unwrap_or_default();
    let mut client = DaemonClient::connect(&paths.socket, settings.daemon.ipc_timeout_secs).await?;

    let deadline = args.deadline.as_ref().and_then(|d| parse_deadline(d));
    let tags = if args.tag.is_empty() {
        None
    } else {
        Some(args.tag)
    };

    let response = client
        .request(Request::UpdateTask {
            id: args.id.clone(),
            title: args.title,
            description: args.description,
            priority: args.priority.map(|p| p.into()),
            tags,
            estimated_minutes: args.estimated,
            deadline,
        })
        .await?;

    match response {
        Response::Task { task } => {
            print_success(&format!("任务已更新: {}", task.short_id()));
            println!("{}", format_task_detail(&task));
        }
        Response::Error { message } => {
            print_error(&message);
        }
        _ => {
            print_error("意外的响应");
        }
    }

    Ok(())
}

async fn handle_feedback(args: cli::FeedbackArgs, paths: &MonoPaths) -> error::Result<()> {
    let settings = Settings::load(&paths.config_file()).unwrap_or_default();
    let mut client = DaemonClient::connect(&paths.socket, settings.daemon.ipc_timeout_secs).await?;

    let response = client
        .request(Request::SubmitFeedback {
            task_id: args.id.clone(),
            rating: args.rating.map(|r| r.min(5)),
            difficulty: args.difficulty.map(|d| d.min(5)),
            energy_level: args.energy.map(|e| e.min(5)),
            notes: args.notes,
        })
        .await?;

    match response {
        Response::Ok => {
            print_success("反馈已提交，将用于优化未来的任务调度");
        }
        Response::Error { message } => {
            print_error(&message);
        }
        _ => {
            print_error("意外的响应");
        }
    }

    Ok(())
}

async fn handle_stats(action: Option<StatsAction>, paths: &MonoPaths) -> error::Result<()> {
    let settings = Settings::load(&paths.config_file()).unwrap_or_default();
    let mut client = DaemonClient::connect(&paths.socket, settings.daemon.ipc_timeout_secs).await?;

    let action = action.unwrap_or(StatsAction::Show {
        task_type: None,
        verbose: false,
    });

    match action {
        StatsAction::Show { task_type, verbose } => {
            handle_stats_show(&mut client, task_type, verbose).await
        }
        StatsAction::Reset { task_type, force } => {
            if !force {
                let target = task_type
                    .as_ref()
                    .map(|t| format!("任务类型 '{}'", t))
                    .unwrap_or_else(|| "全部".to_string());
                print!("确定要重置 {} 的学习数据吗? [y/N] ", target);
                use std::io::Write;
                std::io::stdout().flush().ok();

                let mut input = String::new();
                std::io::stdin().read_line(&mut input).ok();

                if !input.trim().eq_ignore_ascii_case("y") {
                    print_info("已取消");
                    return Ok(());
                }
            }

            let response = client
                .request(Request::ResetLearningData {
                    task_type: task_type.clone(),
                })
                .await?;
            match response {
                Response::Ok => {
                    let target = task_type
                        .map(|t| format!("任务类型 '{}' 的", t))
                        .unwrap_or_else(|| "全部".to_string());
                    print_success(&format!("{}学习数据已重置", target));
                }
                Response::Error { message } => print_error(&message),
                _ => print_error("意外的响应"),
            }
            Ok(())
        }

        StatsAction::SetPreference {
            task_type,
            time_slot,
            strength,
        } => {
            let response = client
                .request(Request::SetTimeSlotPreference {
                    task_type: task_type.clone(),
                    time_slot: time_slot.clone(),
                    strength,
                })
                .await?;

            match response {
                Response::Ok => {
                    let slot_display = format_best_time_slot(
                        &time_slot
                            .to_lowercase()
                            .replace("morning", "Morning")
                            .replace("afternoon", "Afternoon")
                            .replace("evening", "Evening")
                            .replace("night", "Night"),
                    );
                    print_success(&format!(
                        "已设置 '{}' 的偏好时段为 {} (强度: {})",
                        task_type, slot_display, strength
                    ));
                    println!();
                    println!(
                        "{}",
                        "💡 偏好已生效，系统将在该时段优先推荐此类任务".dimmed()
                    );
                }
                Response::Error { message } => print_error(&message),
                _ => print_error("意外的响应"),
            }
            Ok(())
        }

        StatsAction::Export { output } => {
            let response = client.request(Request::ExportLearningData).await?;
            match response {
                Response::LearningDataExport { data } => {
                    if let Some(path) = output {
                        std::fs::write(&path, &data)?;
                        print_success(&format!("学习数据已导出到: {}", path));
                    } else {
                        println!("{}", data);
                    }
                }
                Response::Error { message } => print_error(&message),
                _ => print_error("意外的响应"),
            }
            Ok(())
        }

        StatsAction::Import { file, merge } => {
            let data = std::fs::read_to_string(&file)?;
            let response = client
                .request(Request::ImportLearningData { data, merge })
                .await?;

            match response {
                Response::Ok => {
                    let mode = if merge { "合并" } else { "覆盖" };
                    print_success(&format!("学习数据已从 '{}' {} 导入", file, mode));
                }
                Response::Error { message } => print_error(&message),
                _ => print_error("意外的响应"),
            }
            Ok(())
        }

        StatsAction::Inspect { task_type } => {
            let response = client
                .request(Request::InspectLearningModel { task_type })
                .await?;

            match response {
                Response::LearningModelInspection {
                    global_stats,
                    task_type_models,
                } => {
                    println!("\n{}\n", "🔍 学习模型检查".bold());

                    println!("{}", "全局模型".bold().underline());
                    println!(
                        "  总任务数: {}",
                        global_stats.total_tasks.to_string().green()
                    );
                    println!("  FTRL 权重数: {}", global_stats.ftrl_weights_count);
                    println!();

                    println!("  {}", "全局时段统计:".dimmed());
                    let global_time_rows = vec![
                        TimeSlotRow::new("早晨", &global_stats.time_slots.morning),
                        TimeSlotRow::new("下午", &global_stats.time_slots.afternoon),
                        TimeSlotRow::new("傍晚", &global_stats.time_slots.evening),
                        TimeSlotRow::new("夜间", &global_stats.time_slots.night),
                    ];
                    println!("{}\n", Table::new(global_time_rows).with(Style::rounded()));

                    if task_type_models.is_empty() {
                        print_info("暂无任务类型模型");
                    } else {
                        for model in task_type_models {
                            println!(
                                "{}",
                                format!("📋 任务类型: {}", model.task_type)
                                    .bold()
                                    .underline()
                            );
                            println!("  总调度: {}", model.total_scheduled);
                            println!(
                                "  已完成: {} ({}%)",
                                model.total_completed,
                                if model.total_scheduled > 0 {
                                    (model.total_completed as f64 / model.total_scheduled as f64
                                        * 100.0) as u32
                                } else {
                                    0
                                }
                            );
                            println!("  已推迟: {}", model.total_postponed);
                            println!("  已跳过: {}", model.total_skipped);
                            if let Some(avg) = model.avg_duration_minutes {
                                println!("  平均时长: {:.0} 分钟", avg);
                            }
                            println!("  FTRL 权重数: {}", model.ftrl_weights_count);
                            println!();

                            println!("  {}", "类型专属时段统计:".dimmed());
                            let type_time_rows = vec![
                                TimeSlotRow::new("早晨", &model.time_slots.morning),
                                TimeSlotRow::new("下午", &model.time_slots.afternoon),
                                TimeSlotRow::new("傍晚", &model.time_slots.evening),
                                TimeSlotRow::new("夜间", &model.time_slots.night),
                            ];
                            println!("{}\n", Table::new(type_time_rows).with(Style::rounded()));
                        }
                    }
                }
                Response::Error { message } => print_error(&message),
                _ => print_error("意外的响应"),
            }
            Ok(())
        }
    }
}

async fn handle_stats_show(
    client: &mut DaemonClient,
    task_type: Option<String>,
    verbose: bool,
) -> error::Result<()> {
    let response = client
        .request(Request::GetLearningStats { task_type })
        .await?;

    match response {
        Response::LearningStats { stats } => {
            println!("\n{}\n", "📊 学习统计".bold());
            println!(
                "总学习样本: {}\n",
                stats.total_tasks_learned.to_string().green()
            );

            if stats.task_type_stats.is_empty() {
                print_info("暂无任务类型统计数据，完成更多任务后将显示学习成果");
                println!();
                println!("{}", "💡 提示: 使用 'mono stats manage set-preference' 可以手动设置时段偏好来减少冷启动时间".dimmed());
            } else {
                println!("{}\n", "📌 任务类型统计".bold());

                let task_type_rows: Vec<TaskTypeStatsRow> = stats
                    .task_type_stats
                    .iter()
                    .map(TaskTypeStatsRow::from)
                    .collect();

                println!("{}\n", Table::new(task_type_rows).with(Style::rounded()));
            }

            if verbose {
                println!("{}\n", "⏰ 时段成功率".bold());

                let time_slot_rows = vec![
                    TimeSlotRow::new("早晨 (6-12)", &stats.time_slot_stats.morning),
                    TimeSlotRow::new("下午 (12-18)", &stats.time_slot_stats.afternoon),
                    TimeSlotRow::new("傍晚 (18-22)", &stats.time_slot_stats.evening),
                    TimeSlotRow::new("夜间 (22-6)", &stats.time_slot_stats.night),
                ];

                println!("{}\n", Table::new(time_slot_rows).with(Style::rounded()));
            }
        }
        Response::Error { message } => {
            print_error(&message);
        }
        _ => {
            print_error("意外的响应");
        }
    }

    Ok(())
}

#[derive(Tabled)]
struct TaskTypeStatsRow {
    #[tabled(rename = "类型")]
    task_type: String,
    #[tabled(rename = "已调度")]
    total_scheduled: String,
    #[tabled(rename = "已完成")]
    total_completed: String,
    #[tabled(rename = "已推迟")]
    total_postponed: String,
    #[tabled(rename = "完成率")]
    completion_rate: String,
    #[tabled(rename = "最佳时段")]
    best_time_slot: String,
    #[tabled(rename = "平均时长")]
    avg_duration: String,
}

impl From<&protocol::TaskTypeStatsData> for TaskTypeStatsRow {
    fn from(stats: &protocol::TaskTypeStatsData) -> Self {
        let completion_rate_value = stats.completion_rate * 100.0;
        let completion_rate_str = format!("{:.1}%", completion_rate_value);
        let completion_rate_colored = if completion_rate_value >= 80.0 {
            completion_rate_str.green().to_string()
        } else if completion_rate_value >= 60.0 {
            completion_rate_str.yellow().to_string()
        } else {
            completion_rate_str.red().to_string()
        };

        Self {
            task_type: stats.task_type.clone().bold().to_string(),
            total_scheduled: stats.total_scheduled.to_string(),
            total_completed: stats.total_completed.to_string().green().to_string(),
            total_postponed: if stats.total_postponed > 0 {
                stats.total_postponed.to_string().yellow().to_string()
            } else {
                stats.total_postponed.to_string()
            },
            completion_rate: completion_rate_colored,
            best_time_slot: format_best_time_slot(&stats.best_time_slot),
            avg_duration: stats
                .avg_duration_minutes
                .map(|d| format!("{:.0}分钟", d))
                .unwrap_or_else(|| "-".to_string()),
        }
    }
}

#[derive(Tabled)]
struct TimeSlotRow {
    #[tabled(rename = "时段")]
    time_slot: String,
    #[tabled(rename = "成功")]
    successes: String,
    #[tabled(rename = "失败")]
    failures: String,
    #[tabled(rename = "总计")]
    total: String,
    #[tabled(rename = "成功率")]
    success_rate: String,
    #[tabled(rename = "进度条")]
    bar: String,
}

impl TimeSlotRow {
    fn new(label: &str, detail: &TimeSlotDetail) -> Self {
        let total = detail.successes + detail.failures;
        let bar_width = 20;
        let filled = if total > 0 {
            ((detail.success_rate * bar_width as f64) as usize).min(bar_width)
        } else {
            0
        };
        let empty = bar_width - filled;

        let bar = format!("{}{}", "█".repeat(filled), "░".repeat(empty));
        let success_rate_value = detail.success_rate * 100.0;
        let success_rate_str = format!("{:.1}%", success_rate_value);
        let success_rate_colored = if success_rate_value >= 80.0 {
            success_rate_str.green().to_string()
        } else if success_rate_value >= 60.0 {
            success_rate_str.yellow().to_string()
        } else if success_rate_value > 0.0 {
            success_rate_str.red().to_string()
        } else {
            success_rate_str.dimmed().to_string()
        };

        Self {
            time_slot: label.to_string(),
            successes: detail.successes.to_string().green().to_string(),
            failures: if detail.failures > 0 {
                detail.failures.to_string().red().to_string()
            } else {
                detail.failures.to_string().dimmed().to_string()
            },
            total: total.to_string(),
            success_rate: success_rate_colored,
            bar,
        }
    }
}

fn format_best_time_slot(slot: &str) -> String {
    let text = match slot {
        "Morning" => "早晨",
        "Afternoon" => "下午",
        "Evening" => "傍晚",
        "Night" => "夜间",
        _ => "探索中",
    };
    format!("{}", text)
}

fn handle_config(action: Option<ConfigAction>, paths: &MonoPaths) -> error::Result<()> {
    match action {
        Some(ConfigAction::Path) => {
            println!("配置文件: {}", paths.config_file().display());
            println!("数据目录: {}", paths.data_dir.display());
            println!("数据库:   {}", paths.database.display());
            println!("Socket:   {}", paths.socket.display());
            println!("日志文件: {}", paths.log_file.display());
        }
        Some(ConfigAction::Show) => {
            let settings = Settings::load(&paths.config_file()).unwrap_or_default();
            println!("{}", toml::to_string_pretty(&settings).unwrap_or_default());
        }
        Some(ConfigAction::Init) => {
            let config_file = paths.config_file();
            if config_file.exists() {
                print_warning("配置文件已存在");
            } else {
                paths.ensure_dirs()?;
                let settings = Settings::default();
                settings.save(&config_file)?;
                print_success(&format!("配置文件已创建: {}", config_file.display()));
            }
        }
        None => {
            println!("使用 'mono config --help' 查看可用命令");
        }
    }
    Ok(())
}
