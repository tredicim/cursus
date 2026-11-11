//! The `classic` template: one column, no colours, no icons, standard fonts.
//!
//! The layout is deliberately boring so applicant tracking systems can read
//! it. Entries are grouped into sections in the order their kind first appears
//! in the target; the order inside a section follows the target.

use std::fmt::Write as _;

use cursus_core::{Entry, EntryKind, ResolvedEntry, ResolvedTarget, Template, YearMonth};
use thiserror::Error;

const CLASSIC_TEMPLATE: &str = include_str!("../templates/classic.tex");

#[derive(Debug, Error)]
pub enum RenderError {
    #[error("the `{0}` template is not implemented")]
    UnsupportedTemplate(Template),
}

/// Render a resolved target to a complete LaTeX document.
pub fn render(target: &ResolvedTarget) -> Result<String, RenderError> {
    match target.template {
        Template::Classic => Ok(render_classic(target)),
    }
}

fn render_classic(target: &ResolvedTarget) -> String {
    let mut sections: Vec<(EntryKind, Vec<&ResolvedEntry>)> = Vec::new();
    for entry in &target.entries {
        match sections
            .iter_mut()
            .find(|(kind, _)| *kind == entry.entry.kind)
        {
            Some((_, entries)) => entries.push(entry),
            None => sections.push((entry.entry.kind, vec![entry])),
        }
    }

    let mut body = String::new();
    for (kind, entries) in sections {
        let _ = writeln!(body, "\\cvsection{{{}}}", kind.label());
        for entry in entries {
            render_entry(&mut body, entry);
        }
    }

    CLASSIC_TEMPLATE
        .replace("%%CURSUS_TITLE%%", &escape(&target.name))
        .replace("%%CURSUS_BODY%%", &body)
}

fn render_entry(out: &mut String, resolved: &ResolvedEntry) {
    let entry = &resolved.entry;
    let variant = &resolved.variant;

    let _ = write!(out, "\\noindent{{\\bfseries {}}}", escape(&entry.title));
    if let Some(dates) = format_dates(entry) {
        let _ = write!(out, "\\hfill {dates}");
    }
    let _ = writeln!(out, "\\par");

    let mut meta = Vec::new();
    if let Some(organization) = non_empty(&entry.organization) {
        meta.push(escape(organization));
    }
    if let Some(location) = non_empty(&entry.location) {
        meta.push(escape(location));
    }
    if !meta.is_empty() {
        let _ = writeln!(out, "\\noindent {}\\par", meta.join(" \\textbullet{} "));
    }

    if let Some(summary) = non_empty(&variant.summary) {
        let _ = writeln!(out, "{}\\par", escape(summary));
    }

    if !variant.bullets.is_empty() {
        let _ = writeln!(out, "\\begin{{itemize}}");
        for bullet in &variant.bullets {
            let _ = writeln!(out, "  \\item {}", escape(bullet));
        }
        let _ = writeln!(out, "\\end{{itemize}}");
    }

    if !entry.links.is_empty() {
        let links: Vec<String> = entry
            .links
            .iter()
            .map(|link| format!("\\href{{{}}}{{{}}}", escape(&link.url), escape(&link.label)))
            .collect();
        let _ = writeln!(out, "\\noindent {}\\par", links.join(" \\textbullet{} "));
    }
}

fn format_dates(entry: &Entry) -> Option<String> {
    let start = entry.start.as_ref()?;
    let end = match &entry.end {
        Some(end) => format_month(end),
        // Only roles and studies read naturally as ongoing. An award or a
        // certificate with a start and no end is a single event.
        None if matches!(
            entry.kind,
            EntryKind::Experience | EntryKind::Education | EntryKind::Project
        ) =>
        {
            "Present".to_owned()
        }
        None => return Some(format_month(start)),
    };

    Some(format!("{} -- {}", format_month(start), end))
}

fn format_month(value: &YearMonth) -> String {
    const MONTHS: [&str; 12] = [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ];

    let raw = value.as_str();
    let (year, month) = raw.split_at(4);
    let month = month
        .trim_start_matches('-')
        .parse::<usize>()
        .ok()
        .and_then(|month| MONTHS.get(month.saturating_sub(1)))
        .copied()
        .unwrap_or("Unknown");

    format!("{month} {year}")
}

fn non_empty(value: &Option<String>) -> Option<&str> {
    value.as_deref().filter(|value| !value.trim().is_empty())
}

/// Escape the ten characters that are special in LaTeX text.
///
/// Escaping is intentionally character-by-character and independent of the
/// input, so it cannot accidentally produce syntax from user data.
pub fn escape(input: &str) -> String {
    let mut escaped = String::with_capacity(input.len());

    for character in input.chars() {
        match character {
            '\\' => escaped.push_str("\\textbackslash{}"),
            '&' => escaped.push_str("\\&"),
            '%' => escaped.push_str("\\%"),
            '$' => escaped.push_str("\\$"),
            '#' => escaped.push_str("\\#"),
            '_' => escaped.push_str("\\_"),
            '{' => escaped.push_str("\\{"),
            '}' => escaped.push_str("\\}"),
            '~' => escaped.push_str("\\textasciitilde{}"),
            '^' => escaped.push_str("\\textasciicircum{}"),
            '\n' | '\r' => escaped.push(' '),
            other => escaped.push(other),
        }
    }

    escaped
}
