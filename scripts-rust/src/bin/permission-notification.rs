use claude_hooks::*;
use std::io::{self, Read};

fn main() -> io::Result<()> {
    // 標準入力からJSON読み込み
    let mut input_str = String::new();
    io::stdin().read_to_string(&mut input_str)?;

    let input: HookInput = serde_json::from_str(&input_str)
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

    // ブランチ名のサブタイトル用プレフィックス
    let branch_prefix = branch_name
        .as_ref()
        .map(|b| format!("[{}] ", b))
        .unwrap_or_default();

    // デバッグログ: 受信した通知内容を記録
    debug_log(
        "permission-notification",
        &format!(
            "notification_type={:?}, tool_name={:?}, message={:?}",
            input.notification_type, input.tool_name,
            input.message.as_deref().map(|m| if m.len() > 100 { &m[..100] } else { m })
        ),
    );

    // 通知タイプに応じてメッセージを生成
    let (title, subtitle, message) = match input.notification_type.as_deref() {
        Some("idle_prompt") => {
            // アイドル通知（60秒以上待機）
            let title = format!("Claude Code - 入力待ち ({})", dir_name);
            let subtitle = format!("{}⏱️ アイドル状態", branch_prefix);
            let message = input.message.unwrap_or_else(|| "入力を待っています".to_string());
            (title, subtitle, message)
        }
        Some("permission_prompt") | None => {
            // ツール実行の許可リクエスト（従来の動作）
            if let (Some(tool_name), Some(tool_input)) = (&input.tool_name, &input.tool_input) {
                let (tool_subtitle, message) = build_tool_message(tool_name, tool_input, &input.cwd);
                let title = format!("Claude Code - 確認待ち ({})", dir_name);
                let subtitle = format!("{}{}", branch_prefix, tool_subtitle);
                (title, subtitle, message)
            } else {
                // tool_nameもtool_inputもない場合はスキップ（通知を送らない）
                return Ok(());
            }
        }
        Some(other_type) => {
            // その他の通知タイプ: プラン関連の場合はプラン内容を表示
            let title = format!("Claude Code - 通知 ({})", dir_name);
            let subtitle = format!("{}📢 {}", branch_prefix, other_type);

            // プラン承認系の通知の場合、プラン内容を表示
            let message = if other_type.contains("plan") || other_type.contains("exit") {
                get_plan_summary_for_notification()
                    .unwrap_or_else(|| input.message.unwrap_or_else(|| "通知".to_string()))
            } else {
                input.message.unwrap_or_else(|| "通知".to_string())
            };
            (title, subtitle, message)
        }
    };

    // 通知送信
    send_notification(
        &title,
        &message,
        &subtitle,
        &activation_bundle_id,
        "Glass",
    )?;

    // Slack通知送信
    let slack_title_base = match input.notification_type.as_deref() {
        Some("idle_prompt") => "⏱️ Claude Code - Idle",
        Some("permission_prompt") | None => "🔔 Claude Code - Permission Request",
        _ => "📢 Claude Code - Notification",
    };
    let slack_title = format!("{}{}", slack_title_base, branch_suffix);

    let branch_display = branch_name.as_deref().unwrap_or("N/A");
    let slack_fields = vec![
        ("Session ID", input.session_id.as_str()),
        ("Directory", dir_name.as_str()),
        ("Branch", branch_display),
        ("Type", subtitle.as_str()),
        ("Message", message.as_str()),
    ];

    let iterm2_url = build_iterm2_url_scheme();
    if let Err(err) = post_to_slack_rich(&slack_title, &slack_fields, iterm2_url.as_deref()) {
        eprintln!("Slack notification failed: {}", err);
    }

    Ok(())
}

fn build_tool_message(
    tool_name: &str,
    tool_input: &serde_json::Value,
    cwd: &str,
) -> (String, String) {
    match tool_name {
        "Bash" => {
            let subtitle = "🔧 コマンド実行".to_string();
            let description = tool_input
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let command = tool_input
                .get("command")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            let message = if !description.is_empty() {
                description.to_string()
            } else {
                command.to_string()
            };

            (subtitle, message)
        }
        "Read" => {
            let subtitle = "📖 ファイル読み込み".to_string();
            let file_path = tool_input
                .get("file_path")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let rel_path = get_relative_path(file_path, cwd);
            (subtitle, rel_path)
        }
        "Write" => {
            let subtitle = "✍️ ファイル作成".to_string();
            let file_path = tool_input
                .get("file_path")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let rel_path = get_relative_path(file_path, cwd);
            (subtitle, rel_path)
        }
        "Edit" => {
            let subtitle = "✏️ ファイル編集".to_string();
            let file_path = tool_input
                .get("file_path")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let rel_path = get_relative_path(file_path, cwd);
            (subtitle, rel_path)
        }
        "Grep" => {
            let subtitle = "🔍 コード検索".to_string();
            let pattern = tool_input
                .get("pattern")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let message = format!("パターン: {}", pattern);
            (subtitle, message)
        }
        "Glob" => {
            let subtitle = "🔍 ファイル検索".to_string();
            let pattern = tool_input
                .get("pattern")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let message = format!("パターン: {}", pattern);
            (subtitle, message)
        }
        "Task" => {
            let subtitle = "🤖 エージェント実行".to_string();
            let subagent = tool_input
                .get("subagent_type")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let message = format!("タイプ: {}", subagent);
            (subtitle, message)
        }
        "ExitPlanMode" => {
            let subtitle = "📋 プラン承認待ち".to_string();
            let message = get_plan_summary_for_notification()
                .unwrap_or_else(|| "プランのレビューが必要です".to_string());
            (subtitle, message)
        }
        "AskUserQuestion" => {
            let subtitle = "❓ 質問があります".to_string();
            let message = extract_questions_with_options(tool_input);
            // macOS通知向けに短縮
            let message = if message.len() > 200 {
                format!("{}...", &message[..200])
            } else {
                message
            };
            (subtitle, message)
        }
        _ => {
            let subtitle = "🔧 ツール実行".to_string();
            let message = tool_name.to_string();
            (subtitle, message)
        }
    }
}
