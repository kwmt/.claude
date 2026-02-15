use claude_hooks::*;
use std::io::{self, Read};

fn main() -> io::Result<()> {
    let mut input_str = String::new();
    io::stdin().read_to_string(&mut input_str)?;

    let input: PostToolUseInput = serde_json::from_str(&input_str)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    let dir_name = get_dir_name(&input.cwd);

    // ブランチ名取得
    let branch_name = get_git_branch(&input.cwd);
    let branch_suffix = branch_name
        .as_ref()
        .map(|b| format!(" [{}]", b))
        .unwrap_or_default();
    let branch_display = branch_name.as_deref().unwrap_or("N/A");

    // ~/.claude/plans/ から最新の .md ファイルを取得（lib.rsの共有関数を使用）
    let plan_content = get_latest_plan_content().unwrap_or_else(|| "Plan file not found".to_string());

    let title = format!("📋 Plan Ready for Review{}", branch_suffix);
    let fields = vec![
        ("Session ID", input.session_id.as_str()),
        ("Directory", dir_name.as_str()),
        ("Branch", branch_display),
        ("Plan Content", plan_content.as_str()),
    ];

    let iterm2_url = build_iterm2_url_scheme();
    if let Err(err) = post_to_slack_rich(&title, &fields, iterm2_url.as_deref()) {
        eprintln!("Slack notification failed: {}", err);
    }

    Ok(())
}
