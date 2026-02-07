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

    // Slackに通知
    let title = "🤔 New Claude Prompt";
    let fields = vec![
        ("Session ID", input.session_id.as_str()),
        ("Directory", dir_name.as_str()),
        ("Permission Mode", input.permission_mode.as_str()),
        ("Prompt", input.prompt.as_str()),
    ];

    let iterm2_url = build_iterm2_url_scheme();
    if let Err(err) = post_to_slack_rich(title, &fields, iterm2_url.as_deref()) {
        eprintln!("Slack notification failed: {}", err);
    }

    Ok(())
}
