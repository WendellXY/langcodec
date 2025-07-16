use langcodec::Codec;
use std::fs::File;
use std::io::Write;

/// Run the debug command: read a localization file and output as JSON.
pub fn run_debug_command(input: String, lang: Option<String>, output: Option<String>) {
    // Read the input file
    let mut codec = Codec::new();
    if let Err(e) = codec.read_file_by_extension(&input, lang) {
        eprintln!("Error reading {}: {}", input, e);
        std::process::exit(1);
    }

    // Convert to JSON
    let json = serde_json::to_string_pretty(&*codec.resources).unwrap_or_else(|e| {
        eprintln!("Error serializing to JSON: {}", e);
        std::process::exit(1);
    });

    // Output to file or stdout
    match output {
        Some(output_path) => {
            if let Err(e) =
                File::create(&output_path).and_then(|mut f| f.write_all(json.as_bytes()))
            {
                eprintln!("Error writing to {}: {}", output_path, e);
                std::process::exit(1);
            }
            println!("Debug output written to: {}", output_path);
        }
        None => {
            println!("{}", json);
        }
    }
}
