use claude_hooks::*;
use std::io::{self, Read};

fn main() -> io::Result<()> {
    // 標準入力からJSON読み込み
    let mut input_str = String::new();
    io::stdin().read_to_string(&mut input_str)?;

    // デバッグ: 入力JSONをログに記録
    let _ = std::fs::write("/Users/kwmt/.claude/stop-hook-input.log", &input_str);

    let input: StopHookInput = serde_json::from_str(&input_str)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    // ディレクトリ名取得
    let dir_name = get_dir_name(&input.cwd);

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

    // サブタイトル構築
    let subtitle = format!("📝 {}", user_prompt);

    // 通知送信
    send_notification(
        &format!("Claude Code - タスク完了 ({})", dir_name),
        &assistant_message,
        &subtitle,
        &activation_bundle_id,
        "Funk",
    )?;

    Ok(())
}
