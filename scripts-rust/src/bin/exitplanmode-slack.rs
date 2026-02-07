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

    let iterm2_url = build_iterm2_url_scheme();
    if let Err(err) = post_to_slack_rich(title, &fields, iterm2_url.as_deref()) {
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
