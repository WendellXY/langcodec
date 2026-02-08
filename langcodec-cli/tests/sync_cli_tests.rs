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
