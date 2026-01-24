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

fn extract_questions_with_options(tool_input: &serde_json::Value) -> String {
    let questions = match tool_input.get("questions").and_then(|q| q.as_array()) {
        Some(arr) => arr,
        None => return "N/A".to_string(),
    };

    let mut result = Vec::new();

    for (i, q) in questions.iter().enumerate() {
        let question_text = q
            .get("question")
            .and_then(|v| v.as_str())
            .unwrap_or("N/A");

        let header = q
            .get("header")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let mut question_str = if !header.is_empty() {
            format!("*Q{}: [{}]* {}", i + 1, header, question_text)
        } else {
            format!("*Q{}:* {}", i + 1, question_text)
        };

        // オプションを抽出
        if let Some(options) = q.get("options").and_then(|o| o.as_array()) {
            let option_strs: Vec<String> = options
                .iter()
                .enumerate()
                .filter_map(|(j, opt)| {
                    let label = opt.get("label").and_then(|v| v.as_str())?;
                    let description = opt.get("description").and_then(|v| v.as_str());

                    if let Some(desc) = description {
                        Some(format!("  {}. {} - {}", j + 1, label, desc))
                    } else {
                        Some(format!("  {}. {}", j + 1, label))
                    }
                })
                .collect();

            if !option_strs.is_empty() {
                question_str.push_str("\n");
                question_str.push_str(&option_strs.join("\n"));
            }
        }

        result.push(question_str);
    }

    result.join("\n\n")
}
