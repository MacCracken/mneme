//! Task types — extract and manage tasks from note content.
//!
//! Parses Markdown checkbox syntax (`- [ ]` / `- [x]`) and provides
//! task metadata for kanban/board views.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A task extracted from a note.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: Uuid,
    pub note_id: Uuid,
    pub text: String,
    pub completed: bool,
    pub priority: Priority,
    pub due_date: Option<DateTime<Utc>>,
    pub line_number: usize,
}

/// Task priority level.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    Low,
    #[default]
    Medium,
    High,
    Urgent,
}

/// Kanban column for task board views.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Todo,
    InProgress,
    Done,
    Blocked,
}

/// A kanban board derived from notes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskBoard {
    pub tasks: Vec<Task>,
    pub total: usize,
    pub completed: usize,
    pub pending: usize,
}

/// Extract tasks from Markdown content.
///
/// Parses `- [ ] task text` and `- [x] completed task` patterns.
/// Also detects inline priority markers (`!high`, `!urgent`) and
/// due dates (`@2026-03-15`).
pub fn extract_tasks(note_id: Uuid, content: &str) -> Vec<Task> {
    let mut tasks = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        let trimmed = line.trim();

        let (completed, text) = if let Some(rest) = trimmed
            .strip_prefix("- [x] ")
            .or_else(|| trimmed.strip_prefix("- [X] "))
        {
            (true, rest)
        } else if let Some(rest) = trimmed.strip_prefix("- [ ] ") {
            (false, rest)
        } else {
            continue;
        };

        let (text, priority) = extract_priority(text);
        let (text, due_date) = extract_due_date(&text);

        tasks.push(Task {
            id: Uuid::new_v5(
                &Uuid::NAMESPACE_OID,
                format!("{note_id}:{line_num}").as_bytes(),
            ),
            note_id,
            text,
            completed,
            priority,
            due_date,
            line_number: line_num + 1,
        });
    }

    tasks
}

/// Build a task board from extracted tasks.
pub fn build_board(tasks: Vec<Task>) -> TaskBoard {
    let total = tasks.len();
    let completed = tasks.iter().filter(|t| t.completed).count();
    TaskBoard {
        total,
        completed,
        pending: total - completed,
        tasks,
    }
}

/// Extract priority marker from task text.
fn extract_priority(text: &str) -> (String, Priority) {
    if text.contains("!urgent") {
        (
            text.replace("!urgent", "").trim().to_string(),
            Priority::Urgent,
        )
    } else if text.contains("!high") {
        (
            text.replace("!high", "").trim().to_string(),
            Priority::High,
        )
    } else if text.contains("!low") {
        (
            text.replace("!low", "").trim().to_string(),
            Priority::Low,
        )
    } else {
        (text.to_string(), Priority::Medium)
    }
}

/// Extract due date from task text (format: @YYYY-MM-DD).
fn extract_due_date(text: &str) -> (String, Option<DateTime<Utc>>) {
    let re = regex::Regex::new(r"@(\d{4}-\d{2}-\d{2})").unwrap();
    if let Some(caps) = re.captures(text) {
        let date_str = &caps[1];
        let cleaned = re.replace(text, "").trim().to_string();
        if let Ok(date) = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
            let dt = date.and_hms_opt(23, 59, 59).unwrap().and_utc();
            return (cleaned, Some(dt));
        }
    }
    (text.to_string(), None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_basic_tasks() {
        let id = Uuid::new_v4();
        let content = "# Tasks\n- [ ] Do something\n- [x] Done thing\n- Regular item";
        let tasks = extract_tasks(id, content);
        assert_eq!(tasks.len(), 2);
        assert!(!tasks[0].completed);
        assert!(tasks[1].completed);
        assert_eq!(tasks[0].text, "Do something");
    }

    #[test]
    fn extract_priority_markers() {
        let id = Uuid::new_v4();
        let content = "- [ ] Normal task\n- [ ] Fix bug !urgent\n- [ ] Nice to have !low";
        let tasks = extract_tasks(id, content);
        assert_eq!(tasks[0].priority, Priority::Medium);
        assert_eq!(tasks[1].priority, Priority::Urgent);
        assert_eq!(tasks[2].priority, Priority::Low);
    }

    #[test]
    fn extract_due_dates() {
        let id = Uuid::new_v4();
        let content = "- [ ] Submit report @2026-03-15\n- [ ] No date task";
        let tasks = extract_tasks(id, content);
        assert!(tasks[0].due_date.is_some());
        assert!(tasks[1].due_date.is_none());
    }

    #[test]
    fn build_board_counts() {
        let id = Uuid::new_v4();
        let content = "- [ ] A\n- [x] B\n- [ ] C\n- [x] D";
        let tasks = extract_tasks(id, content);
        let board = build_board(tasks);
        assert_eq!(board.total, 4);
        assert_eq!(board.completed, 2);
        assert_eq!(board.pending, 2);
    }

    #[test]
    fn task_serialization() {
        let task = Task {
            id: Uuid::new_v4(),
            note_id: Uuid::new_v4(),
            text: "Test".into(),
            completed: false,
            priority: Priority::High,
            due_date: None,
            line_number: 1,
        };
        let json = serde_json::to_string(&task).unwrap();
        assert!(json.contains("high"));
    }
}
