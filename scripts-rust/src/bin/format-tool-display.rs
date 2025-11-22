use serde::{Deserialize, Serialize};
use std::io::{self, Read};

#[derive(Deserialize, Debug)]
struct ToolInput {
    #[serde(default)]
    tool_name: Option<String>,
    #[serde(default)]
    tool_input: Option<serde_json::Value>,
}

fn main() -> io::Result<()> {
    // 標準入力からJSON読み込み
    let mut input_str = String::new();
    io::stdin().read_to_string(&mut input_str)?;

    let input: ToolInput = match serde_json::from_str(&input_str) {
        Ok(i) => i,
        Err(_) => return Ok(()), // JSON解析失敗時は何も出力せず終了
    };

    if let Some(tool_name) = input.tool_name {
        let message = format_tool_message(&tool_name, input.tool_input.as_ref());
        println!("{}", message);
    }

    Ok(())
}

fn format_tool_message(tool_name: &str, tool_input: Option<&serde_json::Value>) -> String {
    let (emoji, action) = match tool_name {
        "AskUserQuestion" => ("💬", "ユーザーに質問中"),
        "ExitPlanMode" => ("📋", "プラン提示中"),
        "Bash" => {
            if let Some(input) = tool_input {
                if let Some(desc) = input.get("description").and_then(|v| v.as_str()) {
                    return format!("🔧 コマンド実行: {}", desc);
                } else if let Some(cmd) = input.get("command").and_then(|v| v.as_str()) {
                    // コマンドが長い場合は短縮
                    let short_cmd = if cmd.len() > 50 {
                        format!("{}...", &cmd[..47])
                    } else {
                        cmd.to_string()
                    };
                    return format!("🔧 コマンド実行: {}", short_cmd);
                }
            }
            ("🔧", "コマンド実行")
        }
        "Write" => {
            if let Some(input) = tool_input {
                if let Some(path) = input.get("file_path").and_then(|v| v.as_str()) {
                    let filename = path.split('/').last().unwrap_or(path);
                    return format!("✍️ ファイル作成: {}", filename);
                }
            }
            ("✍️", "ファイル作成")
        }
        "Edit" => {
            if let Some(input) = tool_input {
                if let Some(path) = input.get("file_path").and_then(|v| v.as_str()) {
                    let filename = path.split('/').last().unwrap_or(path);
                    return format!("📝 ファイル編集: {}", filename);
                }
            }
            ("📝", "ファイル編集")
        }
        "Read" => {
            if let Some(input) = tool_input {
                if let Some(path) = input.get("file_path").and_then(|v| v.as_str()) {
                    let filename = path.split('/').last().unwrap_or(path);
                    return format!("📖 ファイル読み込み: {}", filename);
                }
            }
            ("📖", "ファイル読み込み")
        }
        "Grep" => {
            if let Some(input) = tool_input {
                if let Some(pattern) = input.get("pattern").and_then(|v| v.as_str()) {
                    let short_pattern = if pattern.len() > 30 {
                        format!("{}...", &pattern[..27])
                    } else {
                        pattern.to_string()
                    };
                    return format!("🔍 コード検索: {}", short_pattern);
                }
            }
            ("🔍", "コード検索")
        }
        "Glob" => {
            if let Some(input) = tool_input {
                if let Some(pattern) = input.get("pattern").and_then(|v| v.as_str()) {
                    return format!("🔍 ファイル検索: {}", pattern);
                }
            }
            ("🔍", "ファイル検索")
        }
        "Task" => {
            if let Some(input) = tool_input {
                if let Some(desc) = input.get("description").and_then(|v| v.as_str()) {
                    return format!("🤖 エージェント実行: {}", desc);
                }
            }
            ("🤖", "エージェント実行")
        }
        "WebFetch" => {
            if let Some(input) = tool_input {
                if let Some(url) = input.get("url").and_then(|v| v.as_str()) {
                    // URLからドメインを抽出
                    let domain = url
                        .split("://")
                        .nth(1)
                        .and_then(|s| s.split('/').next())
                        .unwrap_or(url);
                    return format!("🌐 Web取得: {}", domain);
                }
            }
            ("🌐", "Web取得")
        }
        "WebSearch" => {
            if let Some(input) = tool_input {
                if let Some(query) = input.get("query").and_then(|v| v.as_str()) {
                    let short_query = if query.len() > 30 {
                        format!("{}...", &query[..27])
                    } else {
                        query.to_string()
                    };
                    return format!("🔎 Web検索: {}", short_query);
                }
            }
            ("🔎", "Web検索")
        }
        "TodoWrite" => ("✅", "TODO更新"),
        "NotebookEdit" => ("📓", "ノートブック編集"),
        "Skill" => ("⚡", "スキル実行"),
        "SlashCommand" => {
            if let Some(input) = tool_input {
                if let Some(cmd) = input.get("command").and_then(|v| v.as_str()) {
                    return format!("⚙️ コマンド実行: {}", cmd);
                }
            }
            ("⚙️", "コマンド実行")
        }
        "BashOutput" => ("📤", "出力取得"),
        "KillShell" => ("⛔", "シェル終了"),
        _ => ("▶️", tool_name),
    };

    format!("{} {}", emoji, action)
}
