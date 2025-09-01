use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_stats_json_on_android_strings() {
    let temp_dir = TempDir::new().unwrap();
    let values_dir = temp_dir.path().join("values");
    fs::create_dir_all(&values_dir).unwrap();
    let input_file = values_dir.join("strings.xml");

    let xml = r#"
        <resources>
            <string name="a">Hello</string>
            <string name="b" translatable="false">Ignored</string>
            <string name="c"></string>
        </resources>
    "#;
    fs::write(&input_file, xml).unwrap();

    let output = Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--",
            "stats",
            "-i",
            input_file.to_str().unwrap(),
            "--json",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "CLI failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let v: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    // Expect 1 language
    assert_eq!(v["summary"]["languages"], 1);
    let langs = v["languages"].as_array().unwrap();
    assert_eq!(langs.len(), 1);
    let by_status = &langs[0]["by_status"];
    assert_eq!(by_status["translated"], 1);
    assert_eq!(by_status["do_not_translate"], 1);
    assert_eq!(by_status["new"], 1);
}

