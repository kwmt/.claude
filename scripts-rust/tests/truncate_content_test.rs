use claude_hooks::truncate_content;

#[test]
fn test_truncate_content_short() {
    let content = "Short content";
    let result = truncate_content(content);
    assert_eq!(result, "Short content");
}

#[test]
fn test_truncate_content_exact_limit() {
    let content = "a".repeat(2800);
    let result = truncate_content(&content);
    assert_eq!(result, content);
}

#[test]
fn test_truncate_content_over_limit() {
    let content = "a".repeat(3000);
    let result = truncate_content(&content);
    assert!(result.ends_with("...\n\n(truncated)"));
    assert!(result.len() < content.len());
    // 2800 + "...\n\n(truncated)".len() = 2800 + 16 = 2816
    assert_eq!(result.len(), 2816);
}

#[test]
fn test_truncate_content_empty() {
    let content = "";
    let result = truncate_content(content);
    assert_eq!(result, "");
}

#[test]
fn test_truncate_content_preserves_content() {
    let content = "# Plan\n\n## Overview\nThis is a test plan.";
    let result = truncate_content(content);
    assert_eq!(result, content);
}
