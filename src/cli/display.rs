use chrono::{DateTime, Local, Utc};
use owo_colors::OwoColorize;

use crate::models::{Priority, Task, TaskStatus};

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

    lines.push(format!("ID:       {}", task.id));
    lines.push(format!("Title:    {}", task.title.bold()));
    lines.push(format!("Status:   {}", format_status(&task.status)));
    lines.push(format!("Priority: {}", format_priority(&task.priority)));

    if let Some(ref desc) = task.description {
        lines.push(format!("Description: {}", desc));
    }

    if !task.tags.is_empty() {
        lines.push(format!("Tags:     {}", task.tags.join(", ")));
    }

    if let Some(mins) = task.estimated_minutes {
        lines.push(format!("Estimated: {}", format_duration(mins)));
    }

    if let Some(mins) = task.actual_minutes {
        lines.push(format!("Actual:    {}", format_duration(mins)));
    }

    if let Some(deadline) = task.deadline {
        lines.push(format!(
            "Deadline:  {} ({})",
            format_datetime(deadline),
            format_relative_time(deadline)
        ));
    }

    if let Some(scheduled) = task.scheduled_at {
        lines.push(format!("Scheduled: {}", format_datetime(scheduled)));
    }

    if let Some(completed) = task.completed_at {
        lines.push(format!("Completed: {}", format_datetime(completed)));
    }

    lines.push(format!("Created:   {}", format_datetime(task.created_at)));

    lines.join("\n")
}

pub fn format_task_list(tasks: &[Task]) -> String {
    if tasks.is_empty() {
        return "No tasks found.".dimmed().to_string();
    }

    tasks
        .iter()
        .map(format_task_short)
        .collect::<Vec<_>>()
        .join("\n")
}

fn format_status(status: &TaskStatus) -> String {
    match status {
        TaskStatus::Pending => "Pending".white().to_string(),
        TaskStatus::InProgress => "In Progress".yellow().to_string(),
        TaskStatus::Completed => "Completed".green().to_string(),
        TaskStatus::Cancelled => "Cancelled".red().to_string(),
        TaskStatus::Postponed => "Postponed".cyan().to_string(),
    }
}

fn format_priority(priority: &Priority) -> String {
    match priority {
        Priority::Low => "Low".dimmed().to_string(),
        Priority::Medium => "Medium".white().to_string(),
        Priority::High => "High".yellow().to_string(),
        Priority::Urgent => "Urgent".red().bold().to_string(),
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
            format!("{}d ago", abs_diff.num_days()).red().to_string()
        } else if abs_diff.num_hours() > 0 {
            format!("{}h ago", abs_diff.num_hours()).red().to_string()
        } else {
            format!("{}m ago", abs_diff.num_minutes()).red().to_string()
        }
    } else if diff.num_days() > 0 {
        format!("in {}d", diff.num_days())
    } else if diff.num_hours() > 0 {
        format!("in {}h", diff.num_hours())
    } else {
        format!("in {}m", diff.num_minutes())
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
