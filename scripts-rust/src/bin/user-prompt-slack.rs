use claude_hooks::*;
use std::io::{self, Read};

fn main() -> io::Result<()> {
    // stdinからJSONを読み込み
    let mut input_str = String::new();
    io::stdin().read_to_string(&mut input_str)?;

    let input: UserPromptSubmitInput = serde_json::from_str(&input_str)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    // ディレクトリ名を取得
    let dir_name = get_dir_name(&input.cwd);

    // プロンプトを切り詰め（Slackの制限対応）
    let truncated_prompt = if input.prompt.chars().count() > 200 {
        let truncated: String = input.prompt.chars().take(200).collect();
        format!("{}...", truncated)
    } else {
        input.prompt.clone()
    };

    // Slackに通知
    let title = "🤔 New Claude Prompt";
    let fields = vec![
        ("Directory", dir_name.as_str()),
        ("Permission Mode", input.permission_mode.as_str()),
        ("Prompt", truncated_prompt.as_str()),
    ];

    if let Err(err) = post_to_slack_rich(title, &fields) {
        eprintln!("Slack notification failed: {}", err);
    }

    Ok(())
}
