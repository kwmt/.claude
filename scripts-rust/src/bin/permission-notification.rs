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

    // ツール別メッセージ生成
    let (subtitle, message) = build_tool_message(&input.tool_name, &input.tool_input, &input.cwd);

    // 通知送信
    send_notification(
        &format!("Claude Code - 確認待ち ({})", dir_name),
        &message,
        &subtitle,
        &activation_bundle_id,
        "Glass",
    )?;

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
                description
            } else {
                command
            };

            let truncated_message = if message.chars().count() > 150 {
                let truncated: String = message.chars().take(150).collect();
                truncated
            } else {
                message.to_string()
            };

            (subtitle, truncated_message)
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
