use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn langcodec_cmd() -> Command {
    Command::new(assert_cmd::cargo::cargo_bin!("langcodec"))
}

#[test]
fn test_cli_view_android_plurals() {
    let temp_dir = TempDir::new().unwrap();
    let input_file = temp_dir.path().join("strings.xml");

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
        .args(["view", "-i", input_file.to_str().unwrap(), "--full"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "CLI failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Check plural structure is printed
    assert!(stdout.contains("Type: Plural"));
    assert!(stdout.contains("Plural ID: apples"));
    assert!(stdout.contains("One apple") || stdout.contains("one"));
    assert!(stdout.contains("%d apples") || stdout.contains("other"));
}
