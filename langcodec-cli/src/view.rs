use atty::Stream;
use crossterm::terminal::size;
use langcodec::Codec;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// Print a view of the localization data, adapting to terminal or pipe output.
pub fn print_view(codec: &Codec, lang: &Option<String>, full: bool) {
    if is_terminal() {
        let term_width = get_terminal_width();
        let key_width = 24;
        let value_width = if term_width > key_width + 16 {
            term_width - key_width - 1
        } else {
            16
        };

        println!("{:<key_width$} {}", "Key", "Value", key_width = key_width);
        for resource in &*codec.resources {
            if let Some(lang) = lang {
                if !resource.has_language(lang) {
                    continue;
                }
            }
            for entry in &resource.entries {
                let key = if full {
                    entry.id.to_string()
                } else {
                    truncate_display(&entry.id, key_width)
                };
                let value = if full {
                    entry.value.plain_translation_string()
                } else {
                    truncate_display(&entry.value.plain_translation_string(), value_width)
                };
                println!("{:<key_width$} {}", key, value, key_width = key_width);
            }
        }
    } else {
        for resource in &*codec.resources {
            if let Some(lang) = lang {
                if !resource.has_language(lang) {
                    continue;
                }
            }
            for entry in &resource.entries {
                println!("{}\t{}", entry.id, entry.value);
            }
        }
    }
}

fn is_terminal() -> bool {
    atty::is(Stream::Stdout)
}

fn get_terminal_width() -> usize {
    size().map(|(w, _)| w as usize).unwrap_or(80)
}

fn truncate_display(s: &str, max: usize) -> String {
    if s.width() <= max {
        s.to_string()
    } else if max > 3 {
        let mut width = 0;
        let mut result = String::new();
        for c in s.chars() {
            let cw = c.width().unwrap_or(0);
            if width + cw > max - 3 {
                break;
            }
            result.push(c);
            width += cw;
        }
        result.push_str("...");
        result
    } else {
        ".".repeat(max)
    }
}
