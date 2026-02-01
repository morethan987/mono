use chrono::{DateTime, Local, Utc};
use owo_colors::OwoColorize;
use tabled::{Table, Tabled, settings::Style};

use crate::models::{Priority, Task, TaskStatus};
use crate::protocol::RankedTask;

#[derive(Tabled)]
struct TaskRow {
    #[tabled(rename = "ID")]
    id: String,
    #[tabled(rename = "状态")]
    status: String,
    #[tabled(rename = "优先级")]
    priority: String,
    #[tabled(rename = "标题")]
    title: String,
    #[tabled(rename = "用时")]
    duration: String,
    #[tabled(rename = "截止")]
    deadline: String,
}

impl From<&Task> for TaskRow {
    fn from(task: &Task) -> Self {
        Self {
            id: task.short_id().to_string(),
            status: status_icon(&task.status),
            priority: priority_icon(&task.priority),
            title: truncate_str(&task.title, 40),
            duration: task
                .estimated_minutes
                .map(format_duration)
                .unwrap_or_else(|| "-".to_string()),
            deadline: task
                .deadline
                .map(format_relative_time)
                .unwrap_or_else(|| "-".to_string()),
        }
    }
}

#[derive(Tabled)]
struct RankedTaskRow {
    #[tabled(rename = "ID")]
    id: String,
    #[tabled(rename = "优先级")]
    priority: String,
    #[tabled(rename = "标题")]
    title: String,
    #[tabled(rename = "截止")]
    deadline: String,
    #[tabled(rename = "得分")]
    score: String,
}

impl From<&RankedTask> for RankedTaskRow {
    fn from(ranked: &RankedTask) -> Self {
        Self {
            id: ranked.task.short_id().to_string(),
            priority: priority_icon(&ranked.task.priority),
            title: truncate_str(&ranked.task.title, 40),
            deadline: ranked
                .task
                .deadline
                .map(format_relative_time)
                .unwrap_or_else(|| "-".to_string()),
            score: format!("{:.2}", ranked.score),
        }
    }
}

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.chars().count() > max_len {
        format!("{}...", s.chars().take(max_len - 3).collect::<String>())
    } else {
        s.to_string()
    }
}

fn status_icon(status: &TaskStatus) -> String {
    match status {
        TaskStatus::Pending => "○".to_string(),
        TaskStatus::InProgress => "◐".yellow().to_string(),
        TaskStatus::Completed => "●".green().to_string(),
        TaskStatus::Cancelled => "✕".red().to_string(),
        TaskStatus::Postponed => "◑".cyan().to_string(),
    }
}

fn priority_code(priority: &Priority) -> String {
    match priority {
        Priority::Low => "L".dimmed().to_string(),
        Priority::Medium => "M".blue().to_string(),
        Priority::High => "H".yellow().to_string(),
        Priority::Urgent => "U".red().bold().to_string(),
    }
}

fn priority_icon(priority: &Priority) -> String {
    match priority {
        Priority::Low => "🟢".to_string(),
        Priority::Medium => "🟡".to_string(),
        Priority::High => "🟠".to_string(),
        Priority::Urgent => "🔴".to_string(),
    }
}

pub fn format_task_short(task: &Task) -> String {
    let status_icon = match task.status {
        TaskStatus::Pending => "○".white().to_string(),
        TaskStatus::InProgress => "◐".yellow().to_string(),
        TaskStatus::Completed => "●".green().to_string(),
        TaskStatus::Cancelled => "✕".red().to_string(),
        TaskStatus::Postponed => "◑".cyan().to_string(),
    };

    let priority_icon = match task.priority {
        Priority::Low => "↓".dimmed().to_string(),
        Priority::Medium => "→".white().to_string(),
        Priority::High => "↑".yellow().to_string(),
        Priority::Urgent => "⚡".red().to_string(),
    };

    let deadline_str = task
        .deadline
        .map(|d| format_relative_time(d))
        .unwrap_or_default();

    let duration_str = task
        .estimated_minutes
        .map(|m| format!("[{}]", format_duration(m)))
        .unwrap_or_default();

    format!(
        "{} {} {} {} {} {}",
        task.short_id().dimmed(),
        status_icon,
        priority_icon,
        task.title,
        duration_str.dimmed(),
        deadline_str.dimmed()
    )
}

pub fn format_task_detail(task: &Task) -> String {
    let mut lines = vec![];

    lines.push(format!("  {}:    {}", "ID".dimmed(), task.short_id()));
    lines.push(format!("  {}:    {}", "标题".dimmed(), task.title.bold()));
    lines.push(format!(
        "  {}:    {}",
        "状态".dimmed(),
        format_status(&task.status)
    ));
    lines.push(format!(
        "  {}:  {}",
        "优先级".dimmed(),
        format_priority(&task.priority)
    ));

    if let Some(ref desc) = task.description {
        lines.push(format!("  {}:    {}", "描述".dimmed(), desc));
    }

    if !task.tags.is_empty() {
        lines.push(format!(
            "  {}:    {}",
            "标签".dimmed(),
            task.tags.join(", ")
        ));
    }

    if let Some(mins) = task.estimated_minutes {
        lines.push(format!(
            "  {}:  {}",
            "预计用时".dimmed(),
            format_duration(mins)
        ));
    }

    if let Some(mins) = task.actual_minutes {
        lines.push(format!(
            "  {}:  {}",
            "实际用时".dimmed(),
            format_duration(mins)
        ));
    }

    if let Some(deadline) = task.deadline {
        lines.push(format!(
            "  {}:  {} ({})",
            "截止时间".dimmed(),
            format_datetime(deadline),
            format_relative_time(deadline)
        ));
    }

    if let Some(scheduled) = task.scheduled_at {
        lines.push(format!(
            "  {}:  {}",
            "计划时间".dimmed(),
            format_datetime(scheduled)
        ));
    }

    if let Some(completed) = task.completed_at {
        lines.push(format!(
            "  {}:  {}",
            "完成时间".dimmed(),
            format_datetime(completed)
        ));
    }

    lines.push(format!(
        "  {}:  {}",
        "创建时间".dimmed(),
        format_datetime(task.created_at)
    ));

    lines.join("\n")
}

pub fn format_task_list(tasks: &[Task]) -> String {
    if tasks.is_empty() {
        return "没有找到任务".dimmed().to_string();
    }

    let rows: Vec<TaskRow> = tasks.iter().map(TaskRow::from).collect();

    Table::new(rows).with(Style::rounded()).to_string()
}

pub fn format_ranked_task_list(tasks: &[RankedTask]) -> String {
    if tasks.is_empty() {
        return "没有待办任务需要规划".dimmed().to_string();
    }

    let rows: Vec<RankedTaskRow> = tasks.iter().map(RankedTaskRow::from).collect();

    Table::new(rows).with(Style::rounded()).to_string()
}

fn format_status(status: &TaskStatus) -> String {
    match status {
        TaskStatus::Pending => "待处理".white().to_string(),
        TaskStatus::InProgress => "进行中".yellow().to_string(),
        TaskStatus::Completed => "已完成".green().to_string(),
        TaskStatus::Cancelled => "已取消".red().to_string(),
        TaskStatus::Postponed => "已推迟".cyan().to_string(),
    }
}

fn format_priority(priority: &Priority) -> String {
    match priority {
        Priority::Low => "低".dimmed().to_string(),
        Priority::Medium => "中".white().to_string(),
        Priority::High => "高".yellow().to_string(),
        Priority::Urgent => "紧急".red().bold().to_string(),
    }
}

fn format_duration(minutes: u32) -> String {
    if minutes >= 60 {
        let hours = minutes / 60;
        let mins = minutes % 60;
        if mins > 0 {
            format!("{}h {}m", hours, mins)
        } else {
            format!("{}h", hours)
        }
    } else {
        format!("{}m", minutes)
    }
}

fn format_datetime(dt: DateTime<Utc>) -> String {
    let local: DateTime<Local> = dt.into();
    local.format("%Y-%m-%d %H:%M").to_string()
}

fn format_relative_time(dt: DateTime<Utc>) -> String {
    let now = Utc::now();
    let diff = dt - now;

    if diff.num_seconds() < 0 {
        let abs_diff = now - dt;
        if abs_diff.num_days() > 0 {
            format!("{}天前", abs_diff.num_days()).red().to_string()
        } else if abs_diff.num_hours() > 0 {
            format!("{}小时前", abs_diff.num_hours()).red().to_string()
        } else {
            format!("{}分钟前", abs_diff.num_minutes())
                .red()
                .to_string()
        }
    } else if diff.num_days() > 0 {
        format!("{}天后", diff.num_days())
    } else if diff.num_hours() > 0 {
        format!("{}小时后", diff.num_hours())
    } else {
        format!("{}分钟后", diff.num_minutes())
    }
}

pub fn print_success(message: &str) {
    println!("{} {}", "✓".green(), message);
}

pub fn print_error(message: &str) {
    eprintln!("{} {}", "✗".red(), message);
}

pub fn print_info(message: &str) {
    println!("{} {}", "ℹ".blue(), message);
}

pub fn print_warning(message: &str) {
    println!("{} {}", "⚠".yellow(), message);
}
