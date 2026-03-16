use atty::Stream;
use clap::builder::styling::{AnsiColor, Effects, Styles};
use crossterm::style::{Attribute, Color, Stylize, style};
use std::fmt::Display;
use unicode_width::UnicodeWidthStr;

#[derive(Clone, Copy)]
pub enum Tone {
    Success,
    Error,
    Warning,
    Info,
    Accent,
    Muted,
}

pub fn clap_styles() -> Styles {
    Styles::styled()
        .header(AnsiColor::Cyan.on_default().effects(Effects::BOLD))
        .usage(AnsiColor::Cyan.on_default().effects(Effects::BOLD))
        .literal(AnsiColor::Yellow.on_default().effects(Effects::BOLD))
        .placeholder(AnsiColor::Green.on_default())
        .valid(AnsiColor::Green.on_default().effects(Effects::BOLD))
        .invalid(AnsiColor::Red.on_default().effects(Effects::BOLD))
        .error(AnsiColor::Red.on_default().effects(Effects::BOLD))
}

fn colors_enabled(stream: Stream) -> bool {
    std::env::var_os("NO_COLOR").is_none() && atty::is(stream)
}

pub fn stdout_styled() -> bool {
    colors_enabled(Stream::Stdout)
}

pub fn stderr_styled() -> bool {
    colors_enabled(Stream::Stderr)
}

fn tone_color(tone: Tone) -> Color {
    match tone {
        Tone::Success => Color::Green,
        Tone::Error => Color::Red,
        Tone::Warning => Color::Yellow,
        Tone::Info => Color::Blue,
        Tone::Accent => Color::Cyan,
        Tone::Muted => Color::DarkGrey,
    }
}

fn tone_label(tone: Tone) -> &'static str {
    match tone {
        Tone::Success => "OK",
        Tone::Error => "ERR",
        Tone::Warning => "WARN",
        Tone::Info => "INFO",
        Tone::Accent => "NOTE",
        Tone::Muted => "··",
    }
}

fn plain_prefix(tone: Tone) -> &'static str {
    match tone {
        Tone::Success => "✅",
        Tone::Error => "❌",
        Tone::Warning => "⚠️",
        Tone::Info => "ℹ️",
        Tone::Accent => "•",
        Tone::Muted => "·",
    }
}

pub fn tone_text(text: &str, tone: Tone) -> String {
    if stdout_styled() {
        format!(
            "{}",
            style(text)
                .with(tone_color(tone))
                .attribute(Attribute::Bold)
        )
    } else {
        text.to_string()
    }
}

pub fn accent(text: &str) -> String {
    if stdout_styled() {
        format!(
            "{}",
            style(text).with(Color::Cyan).attribute(Attribute::Bold)
        )
    } else {
        text.to_string()
    }
}

pub fn muted(text: &str) -> String {
    if stdout_styled() {
        format!("{}", style(text).with(Color::DarkGrey))
    } else {
        text.to_string()
    }
}

pub fn divider(width: usize) -> String {
    if stdout_styled() {
        format!("{}", style("─".repeat(width)).with(Color::DarkGrey))
    } else {
        "-".repeat(width)
    }
}

pub fn header(title: &str) -> String {
    if stdout_styled() {
        format!(
            "{}\n{}",
            accent(title),
            divider(title.chars().count().max(24))
        )
    } else {
        format!("=== {} ===", title)
    }
}

pub fn section(title: &str) -> String {
    if stdout_styled() {
        format!("\n{}", accent(title))
    } else {
        format!("\n=== {} ===", title)
    }
}

pub fn key_value(label: &str, value: impl Display) -> String {
    if stdout_styled() {
        let width = UnicodeWidthStr::width(label).min(18);
        let padding = 18usize.saturating_sub(width).max(1);
        format!("{}{}{}", muted(label), " ".repeat(padding), value)
    } else {
        format!("{label}: {value}")
    }
}

pub fn status_line_stdout(tone: Tone, message: &str) -> String {
    if stdout_styled() {
        format!(
            "{} {}",
            style(format!(" {} ", tone_label(tone)))
                .with(tone_color(tone))
                .attribute(Attribute::Bold),
            message
        )
    } else {
        format!("{} {}", plain_prefix(tone), message)
    }
}

pub fn status_line_stderr(tone: Tone, message: &str) -> String {
    if stderr_styled() {
        format!(
            "{} {}",
            style(format!(" {} ", tone_label(tone)))
                .with(tone_color(tone))
                .attribute(Attribute::Bold),
            message
        )
    } else {
        format!("{} {}", plain_prefix(tone), message)
    }
}

pub fn progress_bar(ratio: f64, width: usize) -> String {
    let clamped = ratio.clamp(0.0, 1.0);
    if stdout_styled() {
        let filled = (clamped * width as f64).round() as usize;
        let empty = width.saturating_sub(filled);
        let left = format!("{}", style("█".repeat(filled)).with(Color::Green));
        let right = format!("{}", style("░".repeat(empty)).with(Color::DarkGrey));
        format!("{left}{right}")
    } else {
        let percent = (clamped * 100.0).round() as usize;
        format!("{percent}%")
    }
}
