use claude_hooks::*;
use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;

fn main() -> io::Result<()> {
    let mut input_str = String::new();
    io::stdin().read_to_string(&mut input_str)?;

    let input: PostToolUseInput = serde_json::from_str(&input_str)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    let dir_name = get_dir_name(&input.cwd);

    // ~/.claude/plans/ から最新の .md ファイルを取得
    let plan_content = get_latest_plan_content().unwrap_or_else(|| "Plan file not found".to_string());

    let title = "📋 Plan Ready for Review";
    let fields = vec![
        ("Session ID", input.session_id.as_str()),
        ("Directory", dir_name.as_str()),
        ("Plan Content", plan_content.as_str()),
    ];

    if let Err(err) = post_to_slack_rich(title, &fields) {
        eprintln!("Slack notification failed: {}", err);
    }

    Ok(())
}

fn get_latest_plan_content() -> Option<String> {
    let home = std::env::var("HOME").ok()?;
    let plans_dir = PathBuf::from(&home).join(".claude/plans");

    if !plans_dir.exists() {
        return None;
    }

    // .md ファイルを取得し、更新日時でソート
    let mut md_files: Vec<_> = fs::read_dir(&plans_dir)
        .ok()?
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .path()
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| s == "md")
                .unwrap_or(false)
        })
        .collect();

    md_files.sort_by_key(|entry| {
        entry
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .map(std::cmp::Reverse)
    });

    // 最新のファイルを読み込み
    let latest_file = md_files.first()?;
    let content = fs::read_to_string(latest_file.path()).ok()?;

    Some(truncate_content(&content))
}

fn truncate_content(content: &str) -> String {
    const MAX_LENGTH: usize = 2800;
    if content.len() > MAX_LENGTH {
        let truncated = &content[..MAX_LENGTH];
        format!("{}...\n\n(truncated)", truncated)
    } else {
        content.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
