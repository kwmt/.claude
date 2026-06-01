use claude_hooks::*;
use std::io::{self, Read};

fn main() -> io::Result<()> {
    // チームメンバーの場合は通知をスキップ
    if is_team_member() {
        return Ok(());
    }

    // 標準入力からJSON読み込み
    let mut input_str = String::new();
    io::stdin().read_to_string(&mut input_str)?;

    let input: StopHookInput = serde_json::from_str(&input_str)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    // ディレクトリ名取得
    let dir_name = get_dir_name(&input.cwd);

    // ブランチ名取得
    let branch_name = get_git_branch(&input.cwd);
    let branch_suffix = branch_name
        .as_ref()
        .map(|b| format!(" [{}]", b))
        .unwrap_or_default();

    // アクティベーション用Bundle ID取得
    let activation_bundle_id = get_activation_bundle_id();

    // ユーザープロンプトとアシスタントメッセージを抽出
    let (user_prompt, assistant_message) = if let Some(ref transcript_path) = input.transcript_path {
        let prompt = extract_user_prompt(transcript_path)
            .unwrap_or_else(|_| "リクエスト".to_string());
        let message = extract_assistant_message(transcript_path)
            .unwrap_or_else(|_| "タスクが完了しました".to_string());

        // デバッグログ出力
        let _ = log_to_file(&prompt, &message);

        (prompt, message)
    } else {
        ("リクエスト".to_string(), "タスクが完了しました".to_string())
    };

    // サブタイトル構築（ブランチ名をサブタイトル先頭に表示）
    let branch_prefix = branch_name
        .as_ref()
        .map(|b| format!("[{}] ", b))
        .unwrap_or_default();
    let subtitle = format!("{}📝 {}", branch_prefix, user_prompt);

    // 通知送信
    send_notification(
        &format!("Claude Code - タスク完了 ({})", dir_name),
        &assistant_message,
        &subtitle,
        &activation_bundle_id,
        "Funk",
    )?;

    // Slack通知送信
    let slack_title = format!("✅ Claude Code - Task Complete{}", branch_suffix);
    let branch_display = branch_name.as_deref().unwrap_or("N/A");
    let slack_fields = vec![
        ("Session ID", input.session_id.as_str()),
        ("Directory", dir_name.as_str()),
        ("Branch", branch_display),
        ("User Prompt", user_prompt.as_str()),
        ("Assistant Response", assistant_message.as_str()),
    ];

    let iterm2_url = build_iterm2_url_scheme();
    if let Err(err) = post_to_slack_rich(&slack_title, &slack_fields, iterm2_url.as_deref()) {
        eprintln!("Slack notification failed: {}", err);
    }

    Ok(())
}
