mod convert;
mod debug;
mod edit;
mod formats;
mod merge;
mod path_glob;
mod stats;
mod sync;
mod transformers;
mod validation;
mod view;

use crate::convert::{ConvertOptions, run_unified_convert_command, try_custom_format_view};
use crate::debug::run_debug_command;
use crate::edit::{EditSetOptions, run_edit_set_command};
use crate::merge::{ConflictStrategy, run_merge_command};
use crate::sync::{SyncOptions, run_sync_command};
use crate::validation::{ValidationContext, validate_context, validate_language_code};
use crate::view::print_view;
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{Shell, generate};

use langcodec::Codec;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    commands: Commands,
}

/// Supported subcommands.
#[derive(Subcommand, Debug)]
enum Commands {
    /// Convert localization files between formats.
    ///
    /// This command automatically detects input and output formats from file extensions.
    /// For JSON files, it will try multiple parsing strategies:
    /// - Standard Resource format (if supported by langcodec)
    /// - JSON key-value pairs (for custom JSON formats)
    Convert {
        /// The input file to process
        #[arg(short, long)]
        input: String,
        /// The output file to write the results to
        #[arg(short, long)]
        output: String,
        /// Optional input format hint (e.g., "json-language-map", "json-array-language-map", "yaml-language-map", "strings", "android")
        #[arg(long)]
        input_format: Option<String>,
        /// Optional output format hint (e.g., "xcstrings", "strings", "android")
        #[arg(long)]
        output_format: Option<String>,
        /// For xcstrings output: override source language (default: en)
        #[arg(long)]
        source_language: Option<String>,
        /// For xcstrings output: override version (default: 1.0)
        #[arg(long)]
        version: Option<String>,
        /// Language codes to exclude from output (e.g., "en", "fr"). Can be specified multiple times or as comma-separated values (e.g., "--exclude-lang en,fr,zh-hans"). Only affects .langcodec output format.
        #[arg(long, value_name = "LANG", value_delimiter = ',')]
        exclude_lang: Vec<String>,
        /// Language codes to include in output (e.g., "en", "fr"). Can be specified multiple times or as comma-separated values (e.g., "--include-lang en,fr,zh-hans"). If specified, only these languages will be included. Only affects .langcodec output format.
        #[arg(long, value_name = "LANG", value_delimiter = ',')]
        include_lang: Vec<String>,
    },

    /// Edit localization files in-place.
    ///
    /// The `set` action unifies add/update/remove:
    /// - If the key does not exist, it is added
    /// - If `--value` is an empty string or omitted, the key is removed
    /// - Otherwise the key is updated
    Edit {
        #[command(subcommand)]
        command: EditCommands,
    },

    /// Sync existing entries from a source file into a target file.
    ///
    /// Behavior:
    /// - Only updates entries that already exist in target
    /// - Never adds new keys to target
    /// - Matches by key first
    /// - Fallback matching by source-language translation (`--match-lang`, default: inferred/en)
    #[command(verbatim_doc_comment)]
    Sync {
        /// Source localization file (A): values are copied from here
        #[arg(short = 's', long)]
        source: String,

        /// Target localization file (B): existing entries are updated here
        #[arg(short = 't', long)]
        target: String,

        /// Optional output path (default: write back to --target)
        #[arg(short, long)]
        output: Option<String>,

        /// Restrict updates to a single target language (e.g., "fr")
        #[arg(short, long)]
        lang: Option<String>,

        /// Language used for translation-based fallback matching (e.g., "en")
        #[arg(long)]
        match_lang: Option<String>,

        /// Preview changes without writing
        #[arg(long, default_value_t = false)]
        dry_run: bool,
    },

    /// View localization files.
    View {
        /// The input file to view
        #[arg(short, long)]
        input: String,

        /// Optional language code to filter entries by
        #[arg(short, long)]
        lang: Option<String>,

        /// Display full value without truncation (even in terminal)
        #[arg(long)]
        full: bool,

        /// Validate plural completeness against CLDR category sets
        #[arg(long, default_value_t = false)]
        check_plurals: bool,
    },

    /// Merge multiple localization files into one output file with automatic format detection and conversion.
    ///
    /// This command intelligently merges multiple localization files, automatically detecting
    /// input formats and converting to the output format based on the file extension.
    /// Supports merging files with the same language and provides conflict resolution strategies.
    Merge {
        /// The input files to merge (supports multiple formats: .strings, .xml, .csv, .tsv, .xcstrings, .json, .yaml)
        #[arg(short, long, num_args = 1.., help = "Input files. Supports glob patterns. Quote patterns to avoid slow shell-side expansion (e.g., '/path/**/*/strings.xml').")]
        inputs: Vec<String>,
        /// The output file path (format automatically determined from extension)
        #[arg(short, long)]
        output: String,
        /// Strategy for handling conflicts when merging entries with the same key
        #[arg(short, long, default_value = "last")]
        strategy: ConflictStrategy,
        /// Language code to use for all input files (e.g., "en", "fr")
        #[arg(short, long)]
        lang: Option<String>,
        /// For xcstrings output: override source language (default: en)
        #[arg(long)]
        source_language: Option<String>,
        /// For xcstrings output: override version (default: 1.0)
        #[arg(long)]
        version: Option<String>,
    },

    /// Show translation coverage and per-status counts.
    Stats {
        /// The input file to analyze
        #[arg(short, long)]
        input: String,
        /// Optional language code to filter by
        #[arg(short, long)]
        lang: Option<String>,
        /// Output JSON instead of human-readable text
        #[arg(long)]
        json: bool,
    },

    /// Debug: Read a localization file and output as JSON.
    Debug {
        /// The input file to debug
        #[arg(short, long)]
        input: String,
        /// Language code to use (e.g., "en", "fr")
        #[arg(short, long)]
        lang: Option<String>,
        /// Output file (defaults to stdout)
        #[arg(short, long)]
        output: Option<String>,
    },

    /// Generate shell completion script and print to stdout.
    ///
    /// Examples:
    /// - langcodec completions bash > /etc/bash_completion.d/langcodec
    /// - langcodec completions zsh > "${fpath[1]}/_langcodec"
    /// - langcodec completions fish > ~/.config/fish/completions/langcodec.fish
    /// - langcodec completions powershell > langcodec.ps1
    Completions {
        /// Shell to generate completions for (bash, zsh, fish, powershell, elvish)
        #[arg(value_enum)]
        shell: Shell,
    },
}

#[derive(Subcommand, Debug)]
enum EditCommands {
    /// Set a key's value (add/update/remove unified).
    ///
    /// Behavior:
    /// - Missing key → add
    /// - Empty or omitted --value → remove
    /// - Otherwise → update
    Set {
        /// The input files to modify (supports glob patterns). Quote patterns to avoid shell expansion.
        #[arg(short, long, num_args = 1.., help = "Input files. Supports glob patterns. Quote patterns to avoid slow shell-side expansion (e.g., '/path/**/*/Localizable.strings').")]
        inputs: Vec<String>,

        /// Language code (required for single-language formats when multiple resources present)
        #[arg(short, long)]
        lang: Option<String>,

        /// Entry key to set
        #[arg(short, long)]
        key: String,

        /// New value. If omitted or empty, the entry will be removed.
        #[arg(short, long)]
        value: Option<String>,

        /// Optional translator comment
        #[arg(long)]
        comment: Option<String>,

        /// Optional status: translated|needs_review|new|do_not_translate|stale
        #[arg(long)]
        status: Option<String>,

        /// Optional output file; if omitted, writes in-place to input
        #[arg(short, long)]
        output: Option<String>,

        /// Preview changes without writing
        #[arg(long, default_value_t = false)]
        dry_run: bool,

        /// Continue processing remaining files when a file fails
        #[arg(long, default_value_t = false)]
        continue_on_error: bool,
    },
}

fn main() {
    let args = Args::parse();

    match args.commands {
        Commands::Convert {
            input,
            output,
            input_format,
            output_format,
            exclude_lang,
            include_lang,
            source_language,
            version,
        } => {
            // Create validation context
            let mut context = ValidationContext::new()
                .with_input_file(input.clone())
                .with_output_file(output.clone());

            if let Some(format) = &input_format {
                context = context.with_input_format(format.clone());
            }
            if let Some(format) = &output_format {
                context = context.with_output_format(format.clone());
            }

            // Validate all inputs
            if let Err(e) = validate_context(&context) {
                eprintln!("❌ Validation failed: {}", e);
                std::process::exit(1);
            }

            run_unified_convert_command(
                input,
                output,
                ConvertOptions {
                    input_format,
                    output_format,
                    source_language,
                    version,
                    exclude_lang,
                    include_lang,
                },
            );
        }
        Commands::Edit { command } => match command {
            EditCommands::Set {
                inputs,
                lang,
                key,
                value,
                comment,
                status,
                output,
                dry_run,
                continue_on_error,
            } => {
                let opts = EditSetOptions {
                    inputs,
                    lang,
                    key,
                    value,
                    comment,
                    status,
                    output,
                    dry_run,
                    continue_on_error,
                };

                if let Err(e) = run_edit_set_command(opts) {
                    eprintln!("❌ Edit failed: {}", e);
                    std::process::exit(1);
                }
            }
        },
        Commands::Sync {
            source,
            target,
            output,
            lang,
            match_lang,
            dry_run,
        } => {
            let mut context = ValidationContext::new()
                .with_input_file(source.clone())
                .with_input_file(target.clone());
            if let Some(output_path) = &output {
                context = context.with_output_file(output_path.clone());
            }
            if let Some(lang_code) = &lang {
                context = context.with_language_code(lang_code.clone());
            }
            if let Err(e) = validate_context(&context) {
                eprintln!("❌ Validation failed: {}", e);
                std::process::exit(1);
            }
            if let Some(match_lang_code) = &match_lang
                && let Err(e) = validate_language_code(match_lang_code)
            {
                eprintln!("❌ Validation failed: {}", e);
                std::process::exit(1);
            }

            if let Err(e) = run_sync_command(SyncOptions {
                source,
                target,
                output,
                lang,
                match_lang,
                dry_run,
            }) {
                eprintln!("❌ Sync failed: {}", e);
                std::process::exit(1);
            }
        }
        Commands::View {
            input,
            lang,
            full,
            check_plurals,
        } => {
            // Create validation context
            let mut context = ValidationContext::new().with_input_file(input.clone());

            if let Some(lang_code) = &lang {
                context = context.with_language_code(lang_code.clone());
            }

            // Validate all inputs
            if let Err(e) = validate_context(&context) {
                eprintln!("❌ Validation failed: {}", e);
                std::process::exit(1);
            }

            // Read the input file using the traditional method
            let mut codec = Codec::new();

            // Try standard format first
            if let Ok(()) = codec.read_file_by_extension(&input, lang.clone()) {
                // Standard format succeeded
            } else if input.ends_with(".json")
                || input.ends_with(".yaml")
                || input.ends_with(".yml")
                || input.ends_with(".langcodec")
            {
                // Try custom format for JSON/YAML/langcodec files
                if let Err(e) = try_custom_format_view(&input, lang.clone(), &mut codec) {
                    eprintln!("Failed to read file: {}", e);
                    std::process::exit(1);
                }
            } else {
                eprintln!("Failed to read file: unsupported format");
                std::process::exit(1);
            }

            print_view(&codec, &lang, full);

            if check_plurals {
                match codec.validate_plurals() {
                    Ok(()) => println!("\n✅ Plural validation passed"),
                    Err(e) => {
                        eprintln!("\n❌ Plural validation failed: {}", e);
                        std::process::exit(2);
                    }
                }
            }
        }
        Commands::Merge {
            inputs,
            output,
            strategy,
            lang,
            source_language,
            version,
        } => {
            // Expand any glob patterns in inputs (e.g., *.strings, **/*.xml)
            println!("Expanding glob patterns in inputs: {:?}", inputs);
            let expanded_inputs = match path_glob::expand_input_globs(&inputs) {
                Ok(list) => list,
                Err(e) => {
                    eprintln!("❌ Failed to expand input patterns: {}", e);
                    std::process::exit(1);
                }
            };

            if expanded_inputs.is_empty() {
                eprintln!("❌ No input files matched the provided patterns");
                std::process::exit(1);
            }

            // Create validation context
            let mut context = ValidationContext::new().with_output_file(output.clone());

            for input in &expanded_inputs {
                context = context.with_input_file(input.clone());
            }

            if let Some(lang_code) = &lang {
                context = context.with_language_code(lang_code.clone());
            }

            // Validate all inputs
            if let Err(e) = validate_context(&context) {
                eprintln!("❌ Validation failed: {}", e);
                std::process::exit(1);
            }

            run_merge_command(
                expanded_inputs,
                output,
                strategy,
                lang,
                source_language,
                version,
            );
        }
        Commands::Debug {
            input,
            lang,
            output,
        } => {
            // Create validation context
            let mut context = ValidationContext::new().with_input_file(input.clone());

            if let Some(lang_code) = &lang {
                context = context.with_language_code(lang_code.clone());
            }
            if let Some(output_path) = &output {
                context = context.with_output_file(output_path.clone());
            }

            // Validate all inputs
            if let Err(e) = validate_context(&context) {
                eprintln!("❌ Validation failed: {}", e);
                std::process::exit(1);
            }

            run_debug_command(input, lang, output);
        }
        Commands::Completions { shell } => {
            let mut cmd = Args::command();
            cmd = cmd.bin_name("langcodec");
            generate(shell, &mut cmd, "langcodec", &mut std::io::stdout());
        }
        Commands::Stats { input, lang, json } => {
            // Validate
            let mut context = ValidationContext::new().with_input_file(input.clone());
            if let Some(l) = &lang {
                context = context.with_language_code(l.clone());
            }
            if let Err(e) = validate_context(&context) {
                eprintln!("❌ Validation failed: {}", e);
                std::process::exit(1);
            }

            // Load file using the same logic as view
            let mut codec = Codec::new();
            if let Ok(()) = codec.read_file_by_extension(&input, lang.clone()) {
                // ok
            } else if input.ends_with(".json")
                || input.ends_with(".yaml")
                || input.ends_with(".yml")
                || input.ends_with(".langcodec")
            {
                if let Err(e) = try_custom_format_view(&input, lang.clone(), &mut codec) {
                    eprintln!("Failed to read file: {}", e);
                    std::process::exit(1);
                }
            } else {
                eprintln!("Failed to read file: unsupported format");
                std::process::exit(1);
            }

            stats::print_stats(&codec, &lang, json);
        }
    }
}
