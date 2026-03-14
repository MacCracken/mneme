//! Note templates — generate structured notes from templates.
//!
//! Provides templates for common note types (daily notes, meeting notes, etc.)
//! with variable substitution.

use std::collections::HashMap;

use chrono::Local;
use serde::{Deserialize, Serialize};

/// A note template definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    pub name: String,
    pub description: String,
    pub title_pattern: String,
    pub content: String,
    pub default_tags: Vec<String>,
    pub variables: Vec<TemplateVariable>,
}

/// A variable in a template.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateVariable {
    pub name: String,
    pub description: String,
    pub default: Option<String>,
}

/// Result of rendering a template.
#[derive(Debug, Clone)]
pub struct RenderedTemplate {
    pub title: String,
    pub content: String,
    pub tags: Vec<String>,
    pub path: Option<String>,
}

/// Render a template with the given variables.
pub fn render_template(template: &Template, vars: &HashMap<String, String>) -> RenderedTemplate {
    let mut context = build_context(vars);

    let title = substitute(&template.title_pattern, &context);
    let content = substitute(&template.content, &context);

    // Add title to context for path generation
    context.insert("title".into(), title.clone());

    let path = if template.title_pattern.contains("{{date}}") {
        // Date-based notes go in year/month subdirectories
        let date = context.get("date").cloned().unwrap_or_default();
        if let Some((year, rest)) = date.split_once('-') {
            if let Some((month, _)) = rest.split_once('-') {
                Some(format!("{year}/{month}/{}.md", slug(&title)))
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    RenderedTemplate {
        title,
        content,
        tags: template.default_tags.clone(),
        path,
    }
}

/// Build variable context with built-in variables.
fn build_context(vars: &HashMap<String, String>) -> HashMap<String, String> {
    let now = Local::now();
    let mut ctx = HashMap::new();

    // Built-in variables
    ctx.insert("date".into(), now.format("%Y-%m-%d").to_string());
    ctx.insert("time".into(), now.format("%H:%M").to_string());
    ctx.insert("datetime".into(), now.format("%Y-%m-%d %H:%M").to_string());
    ctx.insert("year".into(), now.format("%Y").to_string());
    ctx.insert("month".into(), now.format("%m").to_string());
    ctx.insert("day".into(), now.format("%d").to_string());
    ctx.insert("weekday".into(), now.format("%A").to_string());

    // User-provided variables override built-ins
    for (k, v) in vars {
        ctx.insert(k.clone(), v.clone());
    }

    ctx
}

/// Simple `{{variable}}` substitution.
fn substitute(template: &str, vars: &HashMap<String, String>) -> String {
    let mut result = template.to_string();
    for (key, value) in vars {
        result = result.replace(&format!("{{{{{key}}}}}"), value);
    }
    result
}

fn slug(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

// --- Built-in templates ---

/// Daily note template.
pub fn daily_note_template() -> Template {
    Template {
        name: "daily".into(),
        description: "Daily journal / log note".into(),
        title_pattern: "{{date}} — Daily Note".into(),
        content: r#"# {{date}} — {{weekday}}

## Tasks
- [ ]

## Notes


## Log

"#
        .into(),
        default_tags: vec!["daily".into()],
        variables: vec![],
    }
}

/// Meeting note template.
pub fn meeting_note_template() -> Template {
    Template {
        name: "meeting".into(),
        description: "Meeting notes with attendees, agenda, and action items".into(),
        title_pattern: "Meeting: {{topic}} — {{date}}".into(),
        content: r#"# Meeting: {{topic}}

**Date:** {{datetime}}
**Attendees:** {{attendees}}

## Agenda
1.

## Notes


## Action Items
- [ ]

## Decisions

"#
        .into(),
        default_tags: vec!["meeting".into()],
        variables: vec![
            TemplateVariable {
                name: "topic".into(),
                description: "Meeting topic".into(),
                default: Some("Untitled Meeting".into()),
            },
            TemplateVariable {
                name: "attendees".into(),
                description: "Comma-separated list of attendees".into(),
                default: None,
            },
        ],
    }
}

/// Project note template.
pub fn project_note_template() -> Template {
    Template {
        name: "project".into(),
        description: "Project overview note".into(),
        title_pattern: "Project: {{name}}".into(),
        content: r#"# {{name}}

## Overview


## Goals
-

## Status
**Current phase:**
**Started:** {{date}}

## Links

"#
        .into(),
        default_tags: vec!["project".into()],
        variables: vec![TemplateVariable {
            name: "name".into(),
            description: "Project name".into(),
            default: None,
        }],
    }
}

/// Return all built-in templates.
pub fn builtin_templates() -> Vec<Template> {
    vec![
        daily_note_template(),
        meeting_note_template(),
        project_note_template(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_daily_note() {
        let template = daily_note_template();
        let rendered = render_template(&template, &HashMap::new());
        assert!(rendered.title.contains("Daily Note"));
        assert!(rendered.content.contains("## Tasks"));
        assert_eq!(rendered.tags, vec!["daily"]);
        assert!(rendered.path.is_some()); // date-based path
    }

    #[test]
    fn render_meeting_with_vars() {
        let template = meeting_note_template();
        let mut vars = HashMap::new();
        vars.insert("topic".into(), "Sprint Planning".into());
        vars.insert("attendees".into(), "Alice, Bob".into());

        let rendered = render_template(&template, &vars);
        assert!(rendered.title.contains("Sprint Planning"));
        assert!(rendered.content.contains("Alice, Bob"));
    }

    #[test]
    fn render_project_note() {
        let template = project_note_template();
        let mut vars = HashMap::new();
        vars.insert("name".into(), "Mneme".into());

        let rendered = render_template(&template, &vars);
        assert_eq!(rendered.title, "Project: Mneme");
        assert!(rendered.content.contains("# Mneme"));
    }

    #[test]
    fn builtin_template_count() {
        assert_eq!(builtin_templates().len(), 3);
    }

    #[test]
    fn substitute_replaces_vars() {
        let mut vars = HashMap::new();
        vars.insert("name".into(), "World".into());
        assert_eq!(substitute("Hello {{name}}!", &vars), "Hello World!");
    }

    #[test]
    fn substitute_multiple_occurrences() {
        let mut vars = HashMap::new();
        vars.insert("x".into(), "A".into());
        assert_eq!(substitute("{{x}} and {{x}}", &vars), "A and A");
    }

    #[test]
    fn daily_note_path_structure() {
        let template = daily_note_template();
        let rendered = render_template(&template, &HashMap::new());
        let path = rendered.path.unwrap();
        // Should be YYYY/MM/slug.md
        assert!(path.ends_with(".md"));
        assert!(path.contains('/'));
    }
}
