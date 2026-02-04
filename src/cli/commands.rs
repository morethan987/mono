use chrono::{DateTime, Local, NaiveDate, NaiveTime, TimeZone, Utc};
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
    Now(NowArgs),

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

    #[command(about = "开始执行任务")]
    Start(StartArgs),

    #[command(about = "衍生新任务（关联到当前进行中的任务）")]
    Spawn(SpawnArgs),

    #[command(about = "中断当前任务")]
    Interrupt(InterruptArgs),

    #[command(about = "查看学习统计")]
    Stats {
        #[command(subcommand)]
        action: Option<StatsAction>,
    },

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

    #[arg(short, long, help = "优先级（可选，未指定时将自动推断）")]
    pub priority: Option<CliPriority>,

    #[arg(
        short,
        long,
        help = "截止日期 (YYYY-MM-DD [HH:MM] 或 today/tomorrow [HH:MM]，可选)"
    )]
    pub deadline: Option<String>,

    #[arg(short = 't', long, help = "标签（可多次使用，可选）")]
    pub tag: Vec<String>,

    #[arg(short, long, help = "预计时长（分钟，可选，未指定时将自动推断）")]
    pub estimated: Option<u32>,

    #[arg(long, help = "任务描述（可选）")]
    pub description: Option<String>,

    #[arg(short, long, help = "启用智能推断（根据历史自动推断优先级和时长）")]
    pub infer: bool,
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
pub struct StartArgs {
    #[arg(help = "任务 ID（可使用短 ID）")]
    pub id: String,
}

#[derive(Args)]
pub struct SpawnArgs {
    #[arg(help = "衍生任务标题")]
    pub title: String,

    #[arg(short, long, help = "优先级（可选）")]
    pub priority: Option<CliPriority>,

    #[arg(short = 't', long, help = "标签（可多次使用，可选）")]
    pub tag: Vec<String>,

    #[arg(short, long, help = "预计时长（分钟，可选）")]
    pub estimated: Option<u32>,

    #[arg(long, help = "任务描述（可选）")]
    pub description: Option<String>,
}

#[derive(Args)]
pub struct InterruptArgs {
    #[arg(help = "任务 ID（可使用短 ID，默认为当前进行中的任务）")]
    pub id: Option<String>,

    #[arg(short, long, help = "剩余时间（分钟），不指定则自动计算")]
    pub remaining: Option<u32>,
}

#[derive(Args)]
pub struct NowArgs {
    #[arg(long, help = "显示任务队列（前5个）")]
    pub queue: bool,

    #[arg(long, help = "显示所有待办任务")]
    pub all: bool,
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
    #[arg(help = "任务 ID（可使用短 ID，支持多个）", required = true)]
    pub ids: Vec<String>,

    #[arg(short, long, help = "跳过确认")]
    pub force: bool,
}

#[derive(Args)]
pub struct UpdateArgs {
    #[arg(help = "任务 ID（可使用短 ID）")]
    pub id: String,

    #[arg(long, help = "新标题")]
    pub title: Option<String>,

    #[arg(short, long, help = "优先级")]
    pub priority: Option<CliPriority>,

    #[arg(
        short,
        long,
        help = "截止日期 (YYYY-MM-DD [HH:MM] 或 today/tomorrow [HH:MM])"
    )]
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

#[derive(Subcommand)]
pub enum StatsAction {
    #[command(about = "查看学习统计（默认）")]
    Show {
        #[arg(short, long, help = "只显示特定任务类型的统计")]
        task_type: Option<String>,

        #[arg(short, long, help = "显示详细信息")]
        verbose: bool,
    },

    #[command(about = "重置学习数据")]
    Reset {
        #[arg(short, long, help = "重置特定任务类型（不指定则重置全部）")]
        task_type: Option<String>,

        #[arg(short, long, help = "跳过确认")]
        force: bool,
    },

    #[command(about = "设置时段偏好（减少冷启动时间）")]
    SetPreference {
        #[arg(help = "任务类型 (work/study/exercise/default 等)")]
        task_type: String,

        #[arg(help = "偏好时段 (morning/afternoon/evening/night)")]
        time_slot: String,

        #[arg(short, long, default_value = "5", help = "偏好强度 (1-10)")]
        strength: u32,
    },

    #[command(about = "导出学习数据为 JSON")]
    Export {
        #[arg(short, long, help = "输出文件路径（默认输出到 stdout）")]
        output: Option<String>,
    },

    #[command(about = "从 JSON 导入学习数据")]
    Import {
        #[arg(help = "JSON 文件路径")]
        file: String,

        #[arg(short, long, help = "合并而非覆盖现有数据")]
        merge: bool,
    },

    #[command(about = "显示详细的模型参数")]
    Inspect {
        #[arg(short, long, help = "检查特定任务类型")]
        task_type: Option<String>,
    },
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

/// 解析截止日期字符串，默认使用工作时间结束时刻作为截止时间
pub fn parse_deadline(s: &str) -> Option<DateTime<Utc>> {
    parse_deadline_with_work_end(s, 18)
}

/// 解析截止日期字符串，使用指定的工作结束时间（小时）
///
/// 支持的格式：
/// - "today" / "tomorrow" - 当天/次日的工作结束时间
/// - "today HH:MM" / "tomorrow HH:MM" - 当天/次日的指定时间
/// - "YYYY-MM-DD" - 指定日期的工作结束时间
/// - "YYYY-MM-DD HH:MM" - 指定日期和时间
pub fn parse_deadline_with_work_end(s: &str, work_end_hour: u8) -> Option<DateTime<Utc>> {
    let today = Local::now().date_naive();
    let default_end_time = NaiveTime::from_hms_opt(work_end_hour as u32, 0, 0)?;

    let s_lower = s.to_lowercase();
    let s_trimmed = s_lower.trim();

    if s_trimmed.starts_with("today") {
        let remainder = s_trimmed.strip_prefix("today").unwrap().trim();
        let time = if remainder.is_empty() {
            default_end_time
        } else {
            parse_time_hhmm(remainder)?
        };
        let dt = today.and_time(time);
        return Some(Local.from_local_datetime(&dt).single()?.to_utc());
    }

    if s_trimmed.starts_with("tomorrow") {
        let remainder = s_trimmed.strip_prefix("tomorrow").unwrap().trim();
        let tomorrow = today.succ_opt()?;
        let time = if remainder.is_empty() {
            default_end_time
        } else {
            parse_time_hhmm(remainder)?
        };
        let dt = tomorrow.and_time(time);
        return Some(Local.from_local_datetime(&dt).single()?.to_utc());
    }

    if let Some((date_part, time_part)) = s.split_once(' ') {
        if let Ok(date) = NaiveDate::parse_from_str(date_part.trim(), "%Y-%m-%d") {
            if let Some(time) = parse_time_hhmm(time_part.trim()) {
                let dt = date.and_time(time);
                return Some(Local.from_local_datetime(&dt).single()?.to_utc());
            }
        }
    }

    if let Ok(date) = NaiveDate::parse_from_str(s.trim(), "%Y-%m-%d") {
        let dt = date.and_time(default_end_time);
        return Some(Local.from_local_datetime(&dt).single()?.to_utc());
    }

    None
}

fn parse_time_hhmm(s: &str) -> Option<NaiveTime> {
    NaiveTime::parse_from_str(s, "%H:%M").ok()
}
