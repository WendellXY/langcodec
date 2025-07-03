mod view;

use clap::{Parser, Subcommand};
use langcodec::{Codec, convert_auto};

use crate::view::print_view;

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
    }
}
