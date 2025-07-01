use clap::Parser;
use langcodec::convert_auto;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The input file to process
    #[arg(short, long)]
    input: String,
    /// The output file to write the results to
    #[arg(short, long)]
    output: String,
}

fn main() {
    let args = Args::parse();

    if let Err(e) = convert_auto(args.input, args.output) {
        eprintln!("Error: {}", e);
    }
}
