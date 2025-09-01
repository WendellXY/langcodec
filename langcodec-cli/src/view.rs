use langcodec::Codec;

/// Print a view of the resources in a codec.
pub fn print_view(codec: &Codec, lang_filter: &Option<String>, full: bool) {
    println!("Processing resources...");

    // Use the new high-level methods from the lib crate
    let resources = if let Some(lang) = lang_filter {
        // Check if the language exists
        if !codec.languages().any(|l| l == lang) {
            println!("❌ Language not found");
            eprintln!(
                "Language '{}' not found. Available languages: {}",
                lang,
                codec.languages().collect::<Vec<_>>().join(", ")
            );
            std::process::exit(1);
        }

        // Get resources for the specific language
        codec
            .resources
            .iter()
            .filter(|r| r.metadata.language == *lang)
            .collect::<Vec<_>>()
    } else {
        // Get all resources
        codec.resources.iter().collect::<Vec<_>>()
    };

    if resources.is_empty() {
        println!("❌ No resources found");
        if let Some(lang) = lang_filter {
            eprintln!("No resources found for language: {}", lang);
        } else {
            eprintln!("No resources found");
        }
        std::process::exit(1);
    }

    println!("✅ Found {} resource(s)", resources.len());

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
                    println!("    Type: Singular");
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
                    println!("    Type: Plural");
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

    // Show summary using the new high-level methods
    if lang_filter.is_none() {
        println!("\n=== Summary ===");
        println!("Total languages: {}", codec.languages().count());
        println!("Total unique keys: {}", codec.all_keys().count());

        for lang in codec.languages() {
            let count = codec.entry_count(lang);
            println!("  {}: {} entries", lang, count);
        }
    }
}
