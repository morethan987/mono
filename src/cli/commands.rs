use chrono::{DateTime, NaiveDate, NaiveTime, TimeZone, Utc};
use clap::{Args, Parser, Subcommand, ValueEnum};

use crate::models::Priority;

#[derive(Parser)]
#[command(name = "mono")]
#[command(author = "morethan")]
#[command(version)]
#[command(about = "壹刻 - 智能日程规划引擎，让你专注当下", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(about = "守护进程管理")]
    Daemon {
        #[command(subcommand)]
        action: DaemonAction,
    },

    #[command(about = "添加新任务")]
    Add(AddArgs),

    #[command(about = "列出任务")]
    List(ListArgs),

    #[command(about = "查看当前应该做什么")]
    Now,

    #[command(about = "查看今日日程")]
    Today,

    #[command(about = "标记任务完成")]
    Complete(CompleteArgs),

    #[command(about = "推迟任务")]
    Postpone(PostponeArgs),

    #[command(about = "删除任务")]
    Delete(DeleteArgs),

    #[command(about = "更新任务")]
    Update(UpdateArgs),

    #[command(about = "提交详细反馈")]
    Feedback(FeedbackArgs),

    #[command(about = "重新规划日程")]
    Replan,

    #[command(about = "查看学习统计")]
    Stats(StatsArgs),

    #[command(about = "配置管理")]
    Config {
        #[command(subcommand)]
        action: Option<ConfigAction>,
    },
}

#[derive(Subcommand)]
pub enum DaemonAction {
    #[command(about = "启动守护进程")]
    Start {
        #[arg(long, help = "在前台运行（调试用）")]
        foreground: bool,
    },

    #[command(about = "停止守护进程")]
    Stop,

    #[command(about = "查看守护进程状态")]
    Status,
}

#[derive(Args)]
pub struct AddArgs {
    #[arg(help = "任务标题")]
    pub title: String,

    #[arg(short, long, help = "优先级 (low, medium, high, urgent)")]
    pub priority: Option<CliPriority>,

    #[arg(short, long, help = "截止日期 (YYYY-MM-DD 或 today/tomorrow)")]
    pub deadline: Option<String>,

    #[arg(short = 't', long, help = "标签（可多次使用）")]
    pub tag: Vec<String>,

    #[arg(short, long, help = "预计时长（分钟）")]
    pub estimated: Option<u32>,

    #[arg(long, help = "任务描述")]
    pub description: Option<String>,
}

#[derive(Args)]
pub struct ListArgs {
    #[arg(short, long, help = "按状态筛选 (pending, completed, etc.)")]
    pub status: Option<String>,

    #[arg(short = 'n', long, help = "限制数量")]
    pub limit: Option<u32>,

    #[arg(short, long, help = "显示所有任务（包括已完成）")]
    pub all: bool,
}

#[derive(Args)]
pub struct CompleteArgs {
    #[arg(help = "任务 ID（可使用短 ID）")]
    pub id: String,

    #[arg(short, long, help = "实际用时（分钟）")]
    pub time: Option<u32>,

    #[arg(short, long, help = "跳过反馈提示")]
    pub skip_feedback: bool,
}

#[derive(Args)]
pub struct PostponeArgs {
    #[arg(help = "任务 ID（可使用短 ID）")]
    pub id: String,

    #[arg(short, long, default_value = "15", help = "推迟分钟数")]
    pub minutes: u32,
}

#[derive(Args)]
pub struct FeedbackArgs {
    #[arg(help = "任务 ID（可使用短 ID）")]
    pub id: String,

    #[arg(short, long, help = "满意度评分 (1-5)")]
    pub rating: Option<u8>,

    #[arg(short, long, help = "难度评分 (1-5)")]
    pub difficulty: Option<u8>,

    #[arg(short, long, help = "精力水平 (1-5)")]
    pub energy: Option<u8>,

    #[arg(short, long, help = "备注")]
    pub notes: Option<String>,
}

#[derive(Args)]
pub struct DeleteArgs {
    #[arg(help = "任务 ID（可使用短 ID）")]
    pub id: String,

    #[arg(short, long, help = "跳过确认")]
    pub force: bool,
}

#[derive(Args)]
pub struct StatsArgs {
    #[arg(short, long, help = "只显示特定任务类型的统计")]
    pub task_type: Option<String>,

    #[arg(short, long, help = "显示详细信息")]
    pub verbose: bool,
}

#[derive(Args)]
pub struct UpdateArgs {
    #[arg(help = "任务 ID（可使用短 ID）")]
    pub id: String,

    #[arg(long, help = "新标题")]
    pub title: Option<String>,

    #[arg(short, long, help = "优先级 (low, medium, high, urgent)")]
    pub priority: Option<CliPriority>,

    #[arg(short, long, help = "截止日期 (YYYY-MM-DD 或 today/tomorrow)")]
    pub deadline: Option<String>,

    #[arg(short = 't', long, help = "标签（覆盖现有标签）")]
    pub tag: Vec<String>,

    #[arg(short, long, help = "预计时长（分钟）")]
    pub estimated: Option<u32>,

    #[arg(long, help = "任务描述")]
    pub description: Option<String>,
}

#[derive(Subcommand)]
pub enum ConfigAction {
    #[command(about = "显示配置文件路径")]
    Path,

    #[command(about = "显示当前配置")]
    Show,

    #[command(about = "初始化配置文件")]
    Init,
}

#[derive(Clone, Copy, ValueEnum)]
pub enum CliPriority {
    Low,
    Medium,
    High,
    Urgent,
}

impl From<CliPriority> for Priority {
    fn from(p: CliPriority) -> Self {
        match p {
            CliPriority::Low => Priority::Low,
            CliPriority::Medium => Priority::Medium,
            CliPriority::High => Priority::High,
            CliPriority::Urgent => Priority::Urgent,
        }
    }
}

pub fn parse_deadline(s: &str) -> Option<DateTime<Utc>> {
    let today = Utc::now().date_naive();

    match s.to_lowercase().as_str() {
        "today" => {
            let dt = today.and_time(NaiveTime::from_hms_opt(23, 59, 59)?);
            Some(Utc.from_utc_datetime(&dt))
        }
        "tomorrow" => {
            let tomorrow = today.succ_opt()?;
            let dt = tomorrow.and_time(NaiveTime::from_hms_opt(23, 59, 59)?);
            Some(Utc.from_utc_datetime(&dt))
        }
        _ => {
            if let Ok(date) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
                let dt = date.and_time(NaiveTime::from_hms_opt(23, 59, 59)?);
                Some(Utc.from_utc_datetime(&dt))
            } else {
                None
            }
        }
    }
}
