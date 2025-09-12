use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

// Ensure non-ASCII (UTF-8) text survives CLI conversions involving Android <-> .strings
#[test]
fn test_non_ascii_android_to_strings_and_back() {
    let temp_dir = TempDir::new().unwrap();
    let dir = temp_dir.path();

    let android_xml = dir.join("strings.xml");
    let out_strings = dir.join("out.strings");
    let back_xml = dir.join("back.xml");

    // Minimal Android XML with Chinese content (valid XML, real newlines)
    let xml_content = r#"
<resources>
  <string name="some_non_ascii">你好</string>
  
</resources>
"#;
    fs::write(&android_xml, xml_content).unwrap();

    // Resolve langcodec binary path (prefer Cargo-provided path if available)
    let bin_path = option_env!("CARGO_BIN_EXE_langcodec")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            // Fallback to debug target path in workspace
            let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            p.pop(); // leave langcodec-cli, go to workspace root
            p.push("target");
            p.push("debug");
            p.push(if cfg!(windows) {
                "langcodec.exe"
            } else {
                "langcodec"
            });
            p
        });

    // Convert Android -> .strings (invoke binary directly to avoid nested cargo)
    let output = Command::new(&bin_path)
        .args([
            "convert",
            "--input-format",
            "android",
            "--input",
            android_xml.to_str().unwrap(),
            "--output",
            out_strings.to_str().unwrap(),
            "--output-format",
            "strings",
        ])
        .output()
        .expect("failed to run langcodec convert Android -> .strings");
    assert!(
        output.status.success(),
        "Android->strings conversion failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Validate .strings content preserves UTF-8
    let strings_content = fs::read_to_string(&out_strings).unwrap();
    assert!(
        strings_content.contains("你好"),
        "Non-ASCII text was not preserved in .strings: {}",
        strings_content
    );
    assert!(
        !strings_content.contains("ä½\u{20}å¥½") && !strings_content.contains("ä½ å¥½"),
        "Detected mojibake in .strings: {}",
        strings_content
    );

    // Convert back .strings -> Android XML
    let output = Command::new(&bin_path)
        .args([
            "convert",
            "--input-format",
            "strings",
            "--input",
            out_strings.to_str().unwrap(),
            "--output",
            back_xml.to_str().unwrap(),
            "--output-format",
            "android",
        ])
        .output()
        .expect("failed to run langcodec convert .strings -> Android");
    assert!(
        output.status.success(),
        ".strings->Android conversion failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Validate back XML also preserves UTF-8
    let back_xml_content = fs::read_to_string(&back_xml).unwrap();
    assert!(
        back_xml_content.contains("你好"),
        "Non-ASCII text was not preserved in back-converted XML: {}",
        back_xml_content
    );
}
