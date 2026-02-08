use serde_json::Value;
use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_diff_json_reports_added_removed_changed() {
    let temp_dir = TempDir::new().unwrap();
    let source = temp_dir.path().join("source.csv");
    let target = temp_dir.path().join("target.csv");

    let source_content = "\
key,en,fr
a,A1,FA1
b,B1,FB1
c,C1,FC1
";
    let target_content = "\
key,en,fr
b,B2,FB2
c,C1,FC1
d,D1,FD1
";
    fs::write(&source, source_content).unwrap();
    fs::write(&target, target_content).unwrap();

    let out = Command::new("cargo")
        .args([
            "run",
            "--",
            "diff",
            "--source",
            source.to_str().unwrap(),
            "--target",
            target.to_str().unwrap(),
            "--json",
        ])
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    let report: Value = serde_json::from_str(&stdout).unwrap();
    let langs = report["languages"].as_array().unwrap();
    let en = langs
        .iter()
        .find(|l| l["language"] == "en")
        .expect("missing en language report");

    assert_eq!(en["counts"]["added"], 1);
    assert_eq!(en["counts"]["removed"], 1);
    assert_eq!(en["counts"]["changed"], 1);
    assert_eq!(en["counts"]["unchanged"], 1);
}

#[test]
fn test_diff_lang_filter() {
    let temp_dir = TempDir::new().unwrap();
    let source = temp_dir.path().join("source.csv");
    let target = temp_dir.path().join("target.csv");

    let source_content = "\
key,en,fr
a,A1,FA1
";
    let target_content = "\
key,en,fr
a,A2,FA2
";
    fs::write(&source, source_content).unwrap();
    fs::write(&target, target_content).unwrap();

    let out = Command::new("cargo")
        .args([
            "run",
            "--",
            "diff",
            "--source",
            source.to_str().unwrap(),
            "--target",
            target.to_str().unwrap(),
            "--lang",
            "en",
        ])
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Language: en"));
    assert!(!stdout.contains("Language: fr"));
}

#[test]
fn test_diff_json_writes_report_file() {
    let temp_dir = TempDir::new().unwrap();
    let source = temp_dir.path().join("source.csv");
    let target = temp_dir.path().join("target.csv");
    let report = temp_dir.path().join("diff_report.json");

    fs::write(&source, "key,en\na,A\n").unwrap();
    fs::write(&target, "key,en\na,B\n").unwrap();

    let out = Command::new("cargo")
        .args([
            "run",
            "--",
            "diff",
            "--source",
            source.to_str().unwrap(),
            "--target",
            target.to_str().unwrap(),
            "--json",
            "--output",
            report.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(report.exists());

    let content = fs::read_to_string(&report).unwrap();
    let parsed: Value = serde_json::from_str(&content).unwrap();
    assert_eq!(parsed["summary"]["languages"], 1);
}
