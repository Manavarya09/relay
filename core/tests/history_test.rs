#[test]
fn history_empty_when_no_relay_dir() {
    let dir = std::env::temp_dir().join("relay_test_no_history");
    std::fs::create_dir_all(&dir).unwrap();

    let entries = relay::history::list_handoffs(&dir, 10).unwrap();
    assert!(entries.is_empty());

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn history_finds_handoff_files() {
    let dir = std::env::temp_dir().join("relay_test_history");
    let relay_dir = dir.join(".relay");
    std::fs::create_dir_all(&relay_dir).unwrap();

    // Create fake handoff files
    std::fs::write(
        relay_dir.join("handoff_20260405_180000.md"),
        "## CURRENT TASK\n\nFix the bug\n\n  Target agent   : codex\n",
    ).unwrap();

    // Add small delay to ensure different modification times
    std::thread::sleep(std::time::Duration::from_millis(10));

    std::fs::write(
        relay_dir.join("handoff_20260405_190000.md"),
        "## CURRENT TASK\n\nDeploy the app\n\n  Target agent   : gemini\n",
    ).unwrap();

    let entries = relay::history::list_handoffs(&dir, 10).unwrap();
    assert_eq!(entries.len(), 2);
    // Newest first (sorted by file modification time)
    assert!(entries[0].timestamp.contains("19:00"));

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn history_respects_limit() {
    let dir = std::env::temp_dir().join("relay_test_history_limit");
    let relay_dir = dir.join(".relay");
    std::fs::create_dir_all(&relay_dir).unwrap();

    for i in 0..5 {
        std::fs::write(
            relay_dir.join(format!("handoff_20260405_18000{i}.md")),
            format!("## CURRENT TASK\n\nTask {i}\n"),
        ).unwrap();
    }

    let entries = relay::history::list_handoffs(&dir, 3).unwrap();
    assert_eq!(entries.len(), 3);

    std::fs::remove_dir_all(&dir).ok();
}
