use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_sync_updates_existing_entries_with_translation_fallback() {
    let temp_dir = TempDir::new().unwrap();
    let source = temp_dir.path().join("source.csv");
    let target = temp_dir.path().join("target.csv");
    let output = temp_dir.path().join("synced.csv");

    let source_content = "\
key,en,fr
welcome_key,Welcome,Bienvenue
goodbye,Goodbye,Au revoir
new_only,Only in source,Seulement source
";
    let target_content = "\
key,en,fr
Welcome,Old Welcome,Ancienne bienvenue
goodbye,Old Goodbye,Ancien au revoir
keep_me,Keep me,Reste pareil
";

    fs::write(&source, source_content).unwrap();
    fs::write(&target, target_content).unwrap();

    let out = Command::new("cargo")
        .args([
            "run",
            "--",
            "sync",
            "--source",
            source.to_str().unwrap(),
            "--target",
            target.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--match-lang",
            "en",
        ])
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(output.exists());

    let synced = fs::read_to_string(&output).unwrap();
    assert!(synced.contains("Welcome,Welcome,Bienvenue"));
    assert!(synced.contains("goodbye,Goodbye,Au revoir"));
    assert!(synced.contains("keep_me,Keep me,Reste pareil"));
    assert!(!synced.contains("new_only"));
}

#[test]
fn test_sync_dry_run_does_not_write_target() {
    let temp_dir = TempDir::new().unwrap();
    let source = temp_dir.path().join("source.csv");
    let target = temp_dir.path().join("target.csv");

    let source_content = "\
key,en
welcome,Welcome
";
    let target_content = "\
key,en
welcome,Old Welcome
";

    fs::write(&source, source_content).unwrap();
    fs::write(&target, target_content).unwrap();
    let before = fs::read_to_string(&target).unwrap();

    let out = Command::new("cargo")
        .args([
            "run",
            "--",
            "sync",
            "--source",
            source.to_str().unwrap(),
            "--target",
            target.to_str().unwrap(),
            "--dry-run",
            "--match-lang",
            "en",
        ])
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let after = fs::read_to_string(&target).unwrap();
    assert_eq!(before, after);
}

#[test]
fn test_sync_report_json_written() {
    let temp_dir = TempDir::new().unwrap();
    let source = temp_dir.path().join("source.csv");
    let target = temp_dir.path().join("target.csv");
    let report = temp_dir.path().join("sync_report.json");

    fs::write(&source, "key,en\nwelcome,Welcome\n").unwrap();
    fs::write(&target, "key,en\nwelcome,Old Welcome\n").unwrap();

    let out = Command::new("cargo")
        .args([
            "run",
            "--",
            "sync",
            "--source",
            source.to_str().unwrap(),
            "--target",
            target.to_str().unwrap(),
            "--report-json",
            report.to_str().unwrap(),
            "--dry-run",
        ])
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(report.exists());
}

#[test]
fn test_sync_fail_on_unmatched_exits_nonzero() {
    let temp_dir = TempDir::new().unwrap();
    let source = temp_dir.path().join("source.csv");
    let target = temp_dir.path().join("target.csv");

    fs::write(&source, "key,en\nwelcome,Welcome\n").unwrap();
    fs::write(&target, "key,en\nnot_in_source,Old\n").unwrap();

    let out = Command::new("cargo")
        .args([
            "run",
            "--",
            "sync",
            "--source",
            source.to_str().unwrap(),
            "--target",
            target.to_str().unwrap(),
            "--fail-on-unmatched",
            "--dry-run",
        ])
        .output()
        .unwrap();

    assert!(
        !out.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn test_sync_strict_fails_on_unmatched_by_default() {
    let temp_dir = TempDir::new().unwrap();
    let source = temp_dir.path().join("source.csv");
    let target = temp_dir.path().join("target.csv");

    fs::write(&source, "key,en\nwelcome,Welcome\n").unwrap();
    fs::write(&target, "key,en\nnot_in_source,Old\n").unwrap();

    let out = Command::new("cargo")
        .args([
            "run",
            "--",
            "--strict",
            "sync",
            "--source",
            source.to_str().unwrap(),
            "--target",
            target.to_str().unwrap(),
            "--dry-run",
        ])
        .output()
        .unwrap();

    assert!(
        !out.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}
