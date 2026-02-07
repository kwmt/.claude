use serde::Deserialize;
use std::env;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::Command;

// ===== 型定義 =====

#[derive(Deserialize, Debug)]
pub struct HookInput {
    pub session_id: String,
    pub cwd: String,
    #[serde(default)]
    pub tool_name: Option<String>,
    #[serde(default)]
    pub tool_input: Option<serde_json::Value>,
    #[serde(default)]
    pub notification_type: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct StopHookInput {
    pub session_id: String,
    pub transcript_path: Option<String>,
    pub cwd: String,
}

#[derive(Deserialize, Debug)]
pub struct LockFileData {
    pub pid: u32,
    #[serde(rename = "workspaceFolders")]
    pub workspace_folders: Vec<String>,
    #[serde(rename = "ideName")]
    pub ide_name: String,
}

#[derive(Deserialize, Debug)]
pub struct TranscriptMessage {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub message: Option<MessageContent>,
    #[serde(rename = "isMeta")]
    pub is_meta: Option<bool>,
}

#[derive(Deserialize, Debug)]
pub struct MessageContent {
    pub role: String,
    pub content: serde_json::Value,
}

#[derive(Deserialize, Debug)]
pub struct UserPromptSubmitInput {
    pub session_id: String,
    pub transcript_path: Option<String>,
    pub cwd: String,
    pub permission_mode: String,
    pub hook_event_name: String,
    pub prompt: String,
}

#[derive(Deserialize, Debug)]
pub struct PostToolUseInput {
    pub session_id: String,
    pub transcript_path: Option<String>,
    pub cwd: String,
    pub permission_mode: String,
    pub hook_event_name: String,
    pub tool_name: String,
    pub tool_input: serde_json::Value,
    pub tool_response: serde_json::Value,
    pub tool_use_id: String,
}

// ===== ターミナル検出 =====

pub fn detect_terminal_bundle_id() -> Option<String> {
    // 1. TERM_PROGRAM環境変数で検出
    if let Ok(term_program) = env::var("TERM_PROGRAM") {
        match term_program.as_str() {
            "iTerm.app" => return Some("com.googlecode.iterm2".to_string()),
            "Apple_Terminal" => return Some("com.apple.Terminal".to_string()),
            "WarpTerminal" => return Some("dev.warp.Warp-Stable".to_string()),
            "Hyper" => return Some("co.zeit.hyper".to_string()),
            _ => {}
        }
    }

    // 2. ターミナル固有の環境変数で検出
    if env::var("ITERM_SESSION_ID").is_ok() {
        return Some("com.googlecode.iterm2".to_string());
    }
    if env::var("ALACRITTY_SOCKET").is_ok() {
        return Some("io.alacritty.Alacritty".to_string());
    }
    if env::var("KITTY_WINDOW_ID").is_ok() {
        return Some("net.kovidgoyal.kitty".to_string());
    }
    if env::var("WARP_IS_LOCAL_SHELL_SESSION").is_ok() {
        return Some("dev.warp.Warp-Stable".to_string());
    }

    // 3. LC_TERMINAL環境変数で検出
    if let Ok(lc_terminal) = env::var("LC_TERMINAL") {
        match lc_terminal.as_str() {
            "iTerm2" => return Some("com.googlecode.iterm2".to_string()),
            "Terminal" => return Some("com.apple.Terminal".to_string()),
            _ => {}
        }
    }

    // 4. TERM環境変数で推測
    if let Ok(term) = env::var("TERM") {
        match term.as_str() {
            "xterm-kitty" => return Some("net.kovidgoyal.kitty".to_string()),
            "alacritty" => return Some("io.alacritty.Alacritty".to_string()),
            _ => {}
        }
    }

    // 検出できなかった場合
    None
}

// ===== IDE検出 =====

pub fn detect_ide_bundle_id() -> Option<String> {
    let home = env::var("HOME").ok()?;
    let lock_dir = Path::new(&home).join(".claude/ide");

    if !lock_dir.exists() {
        return None;
    }

    // 最新のlockファイルを取得
    let latest_lock = find_latest_lock_file(&lock_dir)?;

    // JSON解析してPID取得
    let file = File::open(&latest_lock).ok()?;
    let lock_data: LockFileData = serde_json::from_reader(file).ok()?;

    // プロセスが実行中か確認
    if !is_process_running(lock_data.pid) {
        return None;
    }

    // Bundle ID取得
    get_bundle_id_from_pid(lock_data.pid)
}

fn find_latest_lock_file(lock_dir: &Path) -> Option<PathBuf> {
    let mut lock_files: Vec<_> = fs::read_dir(lock_dir)
        .ok()?
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .path()
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| s == "lock")
                .unwrap_or(false)
        })
        .collect();

    lock_files.sort_by_key(|entry| {
        entry
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .map(|t| std::cmp::Reverse(t))
    });

    lock_files.first().map(|entry| entry.path())
}

fn is_process_running(pid: u32) -> bool {
    Command::new("ps")
        .args(&["-p", &pid.to_string()])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn get_bundle_id_from_pid(pid: u32) -> Option<String> {
    // ps -p {pid} -o comm= でアプリケーションパスを取得
    let output = Command::new("ps")
        .args(&["-p", &pid.to_string(), "-o", "comm="])
        .output()
        .ok()?;

    let comm = String::from_utf8(output.stdout).ok()?;
    let app_path = comm
        .lines()
        .next()?
        .trim()
        .split("/Contents/MacOS/")
        .next()?
        .to_string();

    if app_path.is_empty() {
        return None;
    }

    // mdls -name kMDItemCFBundleIdentifier でBundle IDを取得
    let output = Command::new("mdls")
        .args(&[
            "-name",
            "kMDItemCFBundleIdentifier",
            &app_path,
        ])
        .output()
        .ok()?;

    let mdls_output = String::from_utf8(output.stdout).ok()?;
    let bundle_id = mdls_output
        .split('"')
        .nth(1)?
        .to_string();

    Some(bundle_id)
}

// ===== 統合検出 =====

pub fn get_activation_bundle_id() -> String {
    // 1. ターミナルが明示的に検出された場合はそれを使用
    if let Some(terminal_id) = detect_terminal_bundle_id() {
        return terminal_id;
    }
    // 2. IDE検出
    if let Some(ide_id) = detect_ide_bundle_id() {
        return ide_id;
    }
    // 3. フォールバック
    "com.apple.Terminal".to_string()
}

// ===== 通知送信 =====

pub fn send_notification(
    title: &str,
    message: &str,
    subtitle: &str,
    bundle_id: &str,
    sound: &str,
) -> io::Result<()> {
    let mut args = vec![
        "-title".to_string(), title.to_string(),
        "-message".to_string(), message.to_string(),
        "-subtitle".to_string(), subtitle.to_string(),
        "-sound".to_string(), sound.to_string(),
    ];

    // iTerm2の場合: -execute で特定セッションに移動
    if bundle_id == "com.googlecode.iterm2" {
        if let Some(execute_cmd) = build_iterm2_activate_command() {
            args.extend(["-execute".to_string(), execute_cmd]);
        }
    }

    // すべての場合: -activate でアプリをアクティブ化
    args.extend(["-activate".to_string(), bundle_id.to_string()]);

    Command::new("terminal-notifier")
        .args(args.iter().map(|s| s.as_str()).collect::<Vec<_>>())
        .output()?;

    Ok(())
}

fn build_iterm2_activate_command() -> Option<String> {
    let session_id = env::var("ITERM_SESSION_ID").ok()?;
    let guid = session_id.split(':').nth(1)?;
    if guid.is_empty() {
        return None;
    }

    // AppleScriptでセッションIDに一致するセッションを選択
    Some(build_iterm2_osascript(guid))
}

fn build_iterm2_osascript(guid: &str) -> String {
    format!(
        r#"osascript -e 'tell application "iTerm2"' -e 'activate' -e 'repeat with w in windows' -e 'tell w' -e 'repeat with t in tabs' -e 'tell t' -e 'repeat with s in sessions' -e 'if id of s is "{}" then' -e 'select' -e 'end if' -e 'end repeat' -e 'end tell' -e 'end repeat' -e 'end tell' -e 'end repeat' -e 'end tell'"#,
        guid
    )
}

fn url_encode(input: &str) -> String {
    let mut encoded = String::new();
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char);
            }
            _ => {
                encoded.push_str(&format!("%{:02X}", byte));
            }
        }
    }
    encoded
}

pub fn build_iterm2_url_scheme() -> Option<String> {
    let session_id = env::var("ITERM_SESSION_ID").ok()?;
    let guid = session_id.split(':').nth(1)?;
    if guid.is_empty() {
        return None;
    }

    let cmd = build_iterm2_osascript(guid);
    Some(format!("iterm2://command?c={}&silent", url_encode(&cmd)))
}

// ===== ユーティリティ =====

pub fn get_dir_name(cwd: &str) -> String {
    Path::new(cwd)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string()
}

pub fn get_relative_path(file_path: &str, cwd: &str) -> String {
    if file_path.starts_with(cwd) {
        file_path.strip_prefix(cwd)
            .and_then(|p| p.strip_prefix("/"))
            .unwrap_or(file_path)
            .to_string()
    } else {
        Path::new(file_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(file_path)
            .to_string()
    }
}

// ===== トランスクリプト解析 =====

pub fn extract_user_prompt(transcript_path: &str) -> io::Result<String> {
    let file = File::open(transcript_path)?;
    let reader = BufReader::new(file);

    let mut messages: Vec<String> = Vec::new();

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        if let Ok(msg) = serde_json::from_str::<TranscriptMessage>(&line) {
            if msg.msg_type == "user" && msg.is_meta != Some(true) {
                if let Some(message_content) = msg.message {
                    if message_content.role == "user" {
                        let content_str = extract_text_content(&message_content.content);
                        if !content_str.is_empty()
                            && !content_str.contains("<command-name>")
                            && !content_str.starts_with("Caveat:")
                            && !content_str.starts_with("[Request interrupted by user for tool use]")
                        {
                            messages.push(content_str);
                        }
                    }
                }
            }
        }
    }

    let prompt = messages
        .last()
        .cloned()
        .unwrap_or_else(|| "リクエスト".to_string());

    Ok(prompt)
}

pub fn extract_assistant_message(transcript_path: &str) -> io::Result<String> {
    let file = File::open(transcript_path)?;
    let reader = BufReader::new(file);

    let mut messages: Vec<String> = Vec::new();

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        if let Ok(msg) = serde_json::from_str::<TranscriptMessage>(&line) {
            if msg.msg_type == "assistant" {
                if let Some(message_content) = msg.message {
                    if message_content.role == "assistant" {
                        let content_str = extract_text_content(&message_content.content);
                        if !content_str.is_empty() {
                            messages.push(content_str);
                        }
                    }
                }
            }
        }
    }

    let message = messages
        .last()
        .cloned()
        .unwrap_or_else(|| "タスクが完了しました".to_string());

    Ok(message)
}

fn extract_text_content(content: &serde_json::Value) -> String {
    match content {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Array(arr) => {
            for item in arr {
                if let Some(obj) = item.as_object() {
                    if obj.get("type").and_then(|v| v.as_str()) == Some("text") {
                        if let Some(text) = obj.get("text").and_then(|v| v.as_str()) {
                            return text.to_string();
                        }
                    }
                }
            }
            String::new()
        }
        _ => String::new(),
    }
}


pub fn log_to_file(user_prompt: &str, assistant_message: &str) -> io::Result<()> {
    use std::fs::OpenOptions;
    use std::io::Write;

    let home = env::var("HOME").map_err(|_| io::Error::new(io::ErrorKind::NotFound, "HOME not set"))?;
    let log_path = Path::new(&home).join(".claude/task-complete.log");

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)?;

    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    writeln!(file, "[{}]", timestamp)?;
    writeln!(file, "  User Prompt: {}", user_prompt)?;
    writeln!(file, "  Assistant: {}", assistant_message)?;
    writeln!(file)?;

    Ok(())
}

// ===== Slack通知 =====

pub fn post_to_slack_rich(title: &str, fields: &[(&str, &str)], button_url: Option<&str>) -> Result<(), String> {
    let webhook_url = match env::var("CLAUDE_CODE_SLACK_WEBHOOK_URL") {
        Ok(url) if !url.is_empty() => url,
        _ => return Ok(()), // 環境変数が設定されていない場合はスキップ
    };

    // Slack Block Kit形式のペイロードを構築
    let mut blocks = Vec::new();

    // ヘッダーブロック
    blocks.push(ureq::json!({
        "type": "header",
        "text": {
            "type": "plain_text",
            "text": title,
        }
    }));

    // フィールドブロック
    if !fields.is_empty() {
        let field_elements: Vec<_> = fields
            .iter()
            .map(|(label, value)| {
                ureq::json!({
                    "type": "mrkdwn",
                    "text": format!("*{}*\n{}", label, value)
                })
            })
            .collect();

        blocks.push(ureq::json!({
            "type": "section",
            "fields": field_elements
        }));
    }

    // iTerm2で開くボタン
    if let Some(url) = button_url {
        blocks.push(ureq::json!({
            "type": "actions",
            "elements": [{
                "type": "button",
                "text": { "type": "plain_text", "text": "iTerm2 で開く" },
                "url": url,
                "action_id": "open_iterm2"
            }]
        }));
    }

    let payload = ureq::json!({
        "blocks": blocks
    });

    ureq::post(&webhook_url)
        .set("Content-Type", "application/json")
        .send_json(payload)
        .map(|_| ())
        .map_err(|e| format!("Slack POST failed: {}", e))
}

// ===== コンテンツ処理 =====

/// コンテンツを指定の長さで切り詰める
pub fn truncate_content(content: &str) -> String {
    const MAX_LENGTH: usize = 2800;
    if content.len() > MAX_LENGTH {
        let truncated = &content[..MAX_LENGTH];
        format!("{}...\n\n(truncated)", truncated)
    } else {
        content.to_string()
    }
}

/// AskUserQuestionのtool_inputから質問とオプションを抽出してフォーマット
pub fn extract_questions_with_options(tool_input: &serde_json::Value) -> String {
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
