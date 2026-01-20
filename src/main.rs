mod cli;
mod config;
mod daemon;
mod error;
mod models;
mod platform;
mod protocol;
mod scheduling;
mod storage;

use clap::Parser;
use tracing_subscriber::EnvFilter;

use crate::cli::{
    Cli, Commands, ConfigAction, DaemonAction, DaemonClient, format_task_detail, format_task_list,
    parse_deadline, print_error, print_info, print_success, print_warning,
};
use crate::config::{MonoPaths, Settings};
use crate::daemon::{daemon_status, run_daemon_background, run_daemon_foreground, stop_daemon};
use crate::models::TaskStatus;
use crate::protocol::{Request, Response};

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
        Commands::Feedback(_args) => {
            print_warning("feedback 命令将在 Phase 4 实现");
            Ok(())
        }
        Commands::Replan => handle_replan(&paths).await,
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
                    {
                        if let Ok(Response::DaemonStatus {
                            uptime_secs,
                            task_count,
                            ..
                        }) = client.request(Request::GetDaemonStatus).await
                        {
                            println!("  运行时间: {}s", uptime_secs);
                            println!("  任务数量: {}", task_count);
                        }
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
                println!("{:<8} {:<6} {:<50} {:>6}", "ID", "优先级", "标题", "得分");
                println!("{}", "-".repeat(80));
                for ranked in &tasks {
                    let priority_str = match ranked.task.priority {
                        models::Priority::Urgent => "🔴",
                        models::Priority::High => "🟠",
                        models::Priority::Medium => "🟡",
                        models::Priority::Low => "🟢",
                    };
                    let title = if ranked.task.title.len() > 48 {
                        format!("{}...", &ranked.task.title[..45])
                    } else {
                        ranked.task.title.clone()
                    };
                    println!(
                        "{:<8} {:<6} {:<50} {:>5.2}",
                        ranked.task.short_id(),
                        priority_str,
                        title,
                        ranked.score
                    );
                }
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
