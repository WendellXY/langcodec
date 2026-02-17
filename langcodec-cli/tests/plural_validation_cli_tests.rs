use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn langcodec_cmd() -> Command {
    Command::new(assert_cmd::cargo::cargo_bin!("langcodec"))
}

#[test]
fn test_cli_view_check_plurals_fails_on_missing() {
    let temp_dir = TempDir::new().unwrap();
    let input_file = temp_dir.path().join("strings.xml");

    // English requires 'one' and 'other'; provide only 'other'
    let xml = r#"
        <resources>
            <plurals name="apples" translatable="true">
                <item quantity="other">%d apples</item>
            </plurals>
        </resources>
    "#;
    fs::write(&input_file, xml).unwrap();

    let output = langcodec_cmd()
        .args([
            "view",
            "-i",
            input_file.to_str().unwrap(),
            "--lang",
            "en",
            "--check-plurals",
        ])
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "CLI unexpectedly succeeded: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Plural validation failed"),
        "stderr: {}",
        stderr
    );
}

#[test]
fn test_cli_view_check_plurals_passes_when_complete() {
    let temp_dir = TempDir::new().unwrap();
    let input_file = temp_dir.path().join("strings.xml");

    // English: provide 'one' and 'other'
    let xml = r#"
        <resources>
            <plurals name="apples" translatable="true">
                <item quantity="one">One apple</item>
                <item quantity="other">%d apples</item>
            </plurals>
        </resources>
    "#;
    fs::write(&input_file, xml).unwrap();

    let output = langcodec_cmd()
        .args([
            "view",
            "-i",
            input_file.to_str().unwrap(),
            "--lang",
            "en",
            "--check-plurals",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "CLI failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("✅ Plural validation passed"));
}
