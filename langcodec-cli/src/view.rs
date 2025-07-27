use indicatif::{ProgressBar, ProgressStyle};
use langcodec::Codec;

/// Print a view of the resources in a codec.
pub fn print_view(codec: &Codec, lang_filter: &Option<String>, full: bool) {
    // Create progress bar
    let progress_bar = ProgressBar::new_spinner();
    progress_bar.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {wide_msg}")
            .unwrap(),
    );

    progress_bar.set_message("Processing resources...");

    let resources = if let Some(lang) = lang_filter {
        codec
            .resources
            .iter()
            .filter(|r| r.metadata.language == *lang)
            .collect::<Vec<_>>()
    } else {
        codec.resources.iter().collect::<Vec<_>>()
    };

    if resources.is_empty() {
        progress_bar.finish_with_message("❌ No resources found");
        if let Some(lang) = lang_filter {
            eprintln!("No resources found for language: {}", lang);
        } else {
            eprintln!("No resources found");
        }
        std::process::exit(1);
    }

    progress_bar.finish_with_message(format!("✅ Found {} resource(s)", resources.len()));

    for (i, resource) in resources.iter().enumerate() {
        println!("\n=== Resource {} ===", i + 1);
        println!("Language: {}", resource.metadata.language);
        println!("Domain: {}", resource.metadata.domain);
        println!("Entries: {}", resource.entries.len());

        for (j, entry) in resource.entries.iter().enumerate() {
            println!("\n  Entry {}: {}", j + 1, entry.id);
            println!("    Status: {:?}", entry.status);

            if let Some(comment) = &entry.comment {
                println!("    Comment: {}", comment);
            }

            match &entry.value {
                langcodec::types::Translation::Singular(value) => {
                    if full {
                        println!("    Value: {}", value);
                    } else {
                        let truncated = if value.len() > 50 {
                            format!("{}...", &value[..50])
                        } else {
                            value.clone()
                        };
                        println!("    Value: {}", truncated);
                    }
                }
                langcodec::types::Translation::Plural(plural) => {
                    println!("    Plural ID: {}", plural.id);
                    for (category, value) in &plural.forms {
                        if full {
                            println!("      {:?}: {}", category, value);
                        } else {
                            let truncated = if value.len() > 50 {
                                format!("{}...", &value[..50])
                            } else {
                                value.clone()
                            };
                            println!("      {:?}: {}", category, truncated);
                        }
                    }
                }
            }
        }
    }
}
