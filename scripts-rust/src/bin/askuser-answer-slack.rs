use claude_hooks::*;
use std::io::{self, Read};

fn main() -> io::Result<()> {
    let mut input_str = String::new();
    io::stdin().read_to_string(&mut input_str)?;

    let input: PostToolUseInput = serde_json::from_str(&input_str)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    let dir_name = get_dir_name(&input.cwd);

    // ブランチ名取得
    let branch_name = get_git_branch(&input.cwd);
    let branch_suffix = branch_name
        .as_ref()
        .map(|b| format!(" [{}]", b))
        .unwrap_or_default();
    let branch_display = branch_name.as_deref().unwrap_or("N/A");

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

    // ユーザー回答を抽出（tool_input.answers → tool_response.answers の順で試行）
    let answer = extract_answer(&input.tool_input, &input.tool_response);

    let title = format!("💬 AskUserQuestion Response{}", branch_suffix);
    let fields = vec![
        ("Session ID", input.session_id.as_str()),
        ("Directory", dir_name.as_str()),
        ("Branch", branch_display),
        ("Question", questions.as_str()),
        ("Answer", answer.as_str()),
    ];

    let iterm2_url = build_iterm2_url_scheme();
    if let Err(err) = post_to_slack_rich(&title, &fields, iterm2_url.as_deref()) {
        eprintln!("Slack notification failed: {}", err);
    }

    Ok(())
}

/// tool_input.answers と tool_response の両方から回答を抽出
fn extract_answer(tool_input: &serde_json::Value, tool_response: &serde_json::Value) -> String {
    // 1. tool_input.answers から抽出（最も構造化されたデータ）
    if let Some(answer) = extract_from_answers_field(tool_input) {
        return answer;
    }

    // 2. tool_response.answers から抽出
    if let Some(answer) = extract_from_answers_field(tool_response) {
        return answer;
    }

    // 3. tool_response が文字列の場合
    if let Some(s) = tool_response.as_str() {
        return s.to_string();
    }

    // 4. tool_response が配列の場合（content blocks 形式）
    if let Some(arr) = tool_response.as_array() {
        let texts: Vec<&str> = arr
            .iter()
            .filter_map(|item| {
                if item.get("type").and_then(|t| t.as_str()) == Some("text") {
                    item.get("text").and_then(|t| t.as_str())
                } else {
                    item.as_str()
                }
            })
            .collect();
        if !texts.is_empty() {
            return texts.join("\n");
        }
    }

    // 5. フォールバック
    tool_response.to_string()
}

/// JSON値の "answers" フィールドから回答文字列を抽出
fn extract_from_answers_field(value: &serde_json::Value) -> Option<String> {
    let answers = value.get("answers")?.as_object()?;
    let extracted: Vec<String> = answers
        .values()
        .map(|v| v.as_str().map(String::from).unwrap_or_else(|| v.to_string()))
        .collect();
    if extracted.is_empty() {
        None
    } else {
        Some(extracted.join(", "))
    }
}
