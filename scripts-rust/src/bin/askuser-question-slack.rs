use claude_hooks::*;
use std::io::{self, Read};

fn main() -> io::Result<()> {
    let mut input_str = String::new();
    io::stdin().read_to_string(&mut input_str)?;

    let input: PostToolUseInput = serde_json::from_str(&input_str)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    let dir_name = get_dir_name(&input.cwd);

    // tool_input から質問とオプションを抽出
    let questions_info = extract_questions_with_options(&input.tool_input);

    let title = "❓ AskUserQuestion";
    let fields = vec![
        ("Session ID", input.session_id.as_str()),
        ("Directory", dir_name.as_str()),
        ("Questions", questions_info.as_str()),
    ];

    if let Err(err) = post_to_slack_rich(title, &fields) {
        eprintln!("Slack notification failed: {}", err);
    }

    Ok(())
}
