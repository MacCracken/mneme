//! Calendar types — daily notes and event linking.

use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A calendar entry linked to a note.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarEntry {
    pub date: NaiveDate,
    pub note_id: Uuid,
    pub title: String,
    pub entry_type: EntryType,
}

/// What type of calendar entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntryType {
    DailyNote,
    MeetingNote,
    Event,
    Deadline,
}

/// A month's calendar view.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonthView {
    pub year: i32,
    pub month: u32,
    pub entries: Vec<CalendarEntry>,
    pub days_with_notes: usize,
}

/// Build calendar entries from a set of notes with dates.
pub fn build_calendar_entries(
    notes: &[(Uuid, String, NaiveDate, EntryType)],
) -> Vec<CalendarEntry> {
    notes
        .iter()
        .map(|(id, title, date, entry_type)| CalendarEntry {
            date: *date,
            note_id: *id,
            title: title.clone(),
            entry_type: *entry_type,
        })
        .collect()
}

/// Group entries into a month view.
pub fn month_view(entries: &[CalendarEntry], year: i32, month: u32) -> MonthView {
    let filtered: Vec<CalendarEntry> = entries
        .iter()
        .filter(|e| e.date.year() == year && e.date.month() == month)
        .cloned()
        .collect();

    let days_with_notes = {
        let mut days: Vec<u32> = filtered.iter().map(|e| e.date.day()).collect();
        days.sort();
        days.dedup();
        days.len()
    };

    MonthView {
        year,
        month,
        entries: filtered,
        days_with_notes,
    }
}

/// Detect entry type from note title.
pub fn detect_entry_type(title: &str) -> EntryType {
    let lower = title.to_lowercase();
    if lower.contains("daily") || lower.contains("journal") {
        EntryType::DailyNote
    } else if lower.contains("meeting") || lower.contains("standup") || lower.contains("sync") {
        EntryType::MeetingNote
    } else if lower.contains("deadline") || lower.contains("due") {
        EntryType::Deadline
    } else {
        EntryType::Event
    }
}

/// Parse a date from a note title (formats: "YYYY-MM-DD", "2026-03-13 — Daily Note").
pub fn parse_date_from_title(title: &str) -> Option<NaiveDate> {
    let re = regex::Regex::new(r"(\d{4}-\d{2}-\d{2})").unwrap();
    re.captures(title)
        .and_then(|caps| NaiveDate::parse_from_str(&caps[1], "%Y-%m-%d").ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_daily_note() {
        assert_eq!(
            detect_entry_type("2026-03-13 — Daily Note"),
            EntryType::DailyNote
        );
        assert_eq!(
            detect_entry_type("Sprint Planning Meeting"),
            EntryType::MeetingNote
        );
        assert_eq!(
            detect_entry_type("Project Deadline"),
            EntryType::Deadline
        );
        assert_eq!(detect_entry_type("Random Note"), EntryType::Event);
    }

    #[test]
    fn parse_date_from_daily() {
        let date = parse_date_from_title("2026-03-13 — Daily Note").unwrap();
        assert_eq!(date, NaiveDate::from_ymd_opt(2026, 3, 13).unwrap());
    }

    #[test]
    fn parse_no_date() {
        assert!(parse_date_from_title("No Date Here").is_none());
    }

    #[test]
    fn month_view_filters() {
        let entries = vec![
            CalendarEntry {
                date: NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
                note_id: Uuid::new_v4(),
                title: "March 1".into(),
                entry_type: EntryType::DailyNote,
            },
            CalendarEntry {
                date: NaiveDate::from_ymd_opt(2026, 3, 15).unwrap(),
                note_id: Uuid::new_v4(),
                title: "March 15".into(),
                entry_type: EntryType::Event,
            },
            CalendarEntry {
                date: NaiveDate::from_ymd_opt(2026, 4, 1).unwrap(),
                note_id: Uuid::new_v4(),
                title: "April 1".into(),
                entry_type: EntryType::DailyNote,
            },
        ];
        let view = month_view(&entries, 2026, 3);
        assert_eq!(view.entries.len(), 2);
        assert_eq!(view.days_with_notes, 2);
    }

    #[test]
    fn build_entries_from_tuples() {
        let data = vec![(
            Uuid::new_v4(),
            "Note 1".to_string(),
            NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
            EntryType::DailyNote,
        )];
        let entries = build_calendar_entries(&data);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].title, "Note 1");
    }
}
