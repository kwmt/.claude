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

    // アクティベーション用Bundle ID取得
    let activation_bundle_id = get_activation_bundle_id();

    // 通知タイプに応じてメッセージを生成
    let (title, subtitle, message) = match input.notification_type.as_deref() {
        Some("idle_prompt") => {
            // アイドル通知（60秒以上待機）
            let title = format!("Claude Code - 入力待ち ({})", dir_name);
            let subtitle = "⏱️ アイドル状態".to_string();
            let message = input.message.unwrap_or_else(|| "入力を待っています".to_string());
            (title, subtitle, message)
        }
        Some("permission_prompt") | None => {
            // ツール実行の許可リクエスト（従来の動作）
            if let (Some(tool_name), Some(tool_input)) = (&input.tool_name, &input.tool_input) {
                let (subtitle, message) = build_tool_message(tool_name, tool_input, &input.cwd);
                let title = format!("Claude Code - 確認待ち ({})", dir_name);
                (title, subtitle, message)
            } else {
                // tool_nameもtool_inputもない場合はスキップ（通知を送らない）
                return Ok(());
            }
        }
        Some(other_type) => {
            // その他の通知タイプ
            let title = format!("Claude Code - 通知 ({})", dir_name);
            let subtitle = format!("📢 {}", other_type);
            let message = input.message.unwrap_or_else(|| "通知".to_string());
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
    let slack_title = match input.notification_type.as_deref() {
        Some("idle_prompt") => "⏱️ Claude Code - Idle",
        Some("permission_prompt") | None => "🔔 Claude Code - Permission Request",
        _ => "📢 Claude Code - Notification",
    };

    let slack_fields = vec![
        ("Session ID", input.session_id.as_str()),
        ("Directory", dir_name.as_str()),
        ("Type", subtitle.as_str()),
        ("Message", message.as_str()),
    ];

    if let Err(err) = post_to_slack_rich(slack_title, &slack_fields) {
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
        _ => {
            let subtitle = "🔧 ツール実行".to_string();
            let message = tool_name.to_string();
            (subtitle, message)
        }
    }
}
