use claude_hooks::*;
use std::io::{self, Read};

fn main() -> io::Result<()> {
    let mut input_str = String::new();
    io::stdin().read_to_string(&mut input_str)?;

    let input: PostToolUseInput = serde_json::from_str(&input_str)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    let dir_name = get_dir_name(&input.cwd);

    // tool_input から質問を抽出
    let questions = input
        .tool_input
        .get("questions")
        .and_then(|q| q.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|q| q.get("question").and_then(|v| v.as_str()))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_else(|| "N/A".to_string());

    // tool_response からユーザー回答を抽出
    let answer = extract_answer_from_response(&input.tool_response);

    let title = "💬 AskUserQuestion Response";
    let fields = vec![
        ("Session ID", input.session_id.as_str()),
        ("Directory", dir_name.as_str()),
        ("Question", questions.as_str()),
        ("Answer", answer.as_str()),
    ];

    let iterm2_url = build_iterm2_url_scheme();
    if let Err(err) = post_to_slack_rich(title, &fields, iterm2_url.as_deref()) {
        eprintln!("Slack notification failed: {}", err);
    }

    Ok(())
}

fn extract_answer_from_response(response: &serde_json::Value) -> String {
    // tool_response の構造に応じて回答を抽出
    // 想定: { "answers": { "question_text": "answer_text" } } 形式
    if let Some(answers) = response.get("answers").and_then(|a| a.as_object()) {
        answers
            .values()
            .filter_map(|v| v.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    } else if let Some(s) = response.as_str() {
        s.to_string()
    } else {
        response.to_string()
    }
}

