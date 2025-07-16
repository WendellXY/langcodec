mod debug;
mod merge;
mod view;

use crate::debug::run_debug_command;
use crate::merge::{ConflictStrategy, run_merge_command};
use crate::view::print_view;
use clap::{Parser, Subcommand};
use langcodec::{Codec, convert_auto};

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
    Convert {
        /// The input file to process
        #[arg(short, long)]
        input: String,
        /// The output file to write the results to
        #[arg(short, long)]
        output: String,
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
    },

    /// Merge multiple localization files of the same format into one output file.
    Merge {
        /// The input files to merge
        #[arg(short, long, num_args = 1..)]
        inputs: Vec<String>,
        /// The output file to write the merged results to
        #[arg(short, long)]
        output: String,
        /// Strategy for handling conflicts
        #[arg(short, long, default_value = "last")]
        strategy: ConflictStrategy,
        /// Language code to use for all input files (e.g., "en", "fr")
        #[arg(short, long)]
        lang: Option<String>,
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
}

fn main() {
    let args = Args::parse();

    match args.commands {
        Commands::Convert { input, output } => {
            // Call the conversion function with the provided input and output files
            if let Err(e) = convert_auto(input, output) {
                eprintln!("Error: {}", e);
            }
        }
        Commands::View { input, lang, full } => {
            // Read the input file and print all the entries
            let mut codec = Codec::new();
            codec
                .read_file_by_extension(input, Option::None)
                .expect("Failed to read file");
            print_view(&codec, &lang, full);
        }
        Commands::Merge {
            inputs,
            output,
            strategy,
            lang,
        } => {
            run_merge_command(inputs, output, strategy, lang);
        }
        Commands::Debug {
            input,
            lang,
            output,
        } => {
            run_debug_command(input, lang, output);
        }
    }
}
