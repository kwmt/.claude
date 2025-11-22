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

// ===== ターミナル検出 =====

pub fn detect_terminal_bundle_id() -> String {
    // 1. TERM_PROGRAM環境変数で検出
    if let Ok(term_program) = env::var("TERM_PROGRAM") {
        match term_program.as_str() {
            "iTerm.app" => return "com.googlecode.iterm2".to_string(),
            "Apple_Terminal" => return "com.apple.Terminal".to_string(),
            "WarpTerminal" => return "dev.warp.Warp-Stable".to_string(),
            "Hyper" => return "co.zeit.hyper".to_string(),
            _ => {}
        }
    }

    // 2. ターミナル固有の環境変数で検出
    if env::var("ITERM_SESSION_ID").is_ok() {
        return "com.googlecode.iterm2".to_string();
    }
    if env::var("ALACRITTY_SOCKET").is_ok() {
        return "io.alacritty.Alacritty".to_string();
    }
    if env::var("KITTY_WINDOW_ID").is_ok() {
        return "net.kovidgoyal.kitty".to_string();
    }
    if env::var("WARP_IS_LOCAL_SHELL_SESSION").is_ok() {
        return "dev.warp.Warp-Stable".to_string();
    }

    // 3. LC_TERMINAL環境変数で検出
    if let Ok(lc_terminal) = env::var("LC_TERMINAL") {
        match lc_terminal.as_str() {
            "iTerm2" => return "com.googlecode.iterm2".to_string(),
            "Terminal" => return "com.apple.Terminal".to_string(),
            _ => {}
        }
    }

    // 4. TERM環境変数で推測
    if let Ok(term) = env::var("TERM") {
        match term.as_str() {
            "xterm-kitty" => return "net.kovidgoyal.kitty".to_string(),
            "alacritty" => return "io.alacritty.Alacritty".to_string(),
            _ => {}
        }
    }

    // 6. フォールバック
    "com.apple.Terminal".to_string()
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
    // IDE優先
    if let Some(ide_id) = detect_ide_bundle_id() {
        return ide_id;
    }

    // ターミナルフォールバック
    detect_terminal_bundle_id()
}

// ===== 通知送信 =====

pub fn send_notification(
    title: &str,
    message: &str,
    subtitle: &str,
    bundle_id: &str,
    sound: &str,
) -> io::Result<()> {
    Command::new("terminal-notifier")
        .args(&[
            "-title",
            title,
            "-message",
            message,
            "-subtitle",
            subtitle,
            "-sound",
            sound,
            "-activate",
            bundle_id,
        ])
        .output()?;

    Ok(())
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

    Ok(truncate_string(&prompt, 100))
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

    Ok(truncate_string(&message, 150))
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

fn truncate_string(s: &str, max_len: usize) -> String {
    if s.chars().count() > max_len {
        let truncated: String = s.chars().take(max_len).collect();
        format!("{}...", truncated)
    } else {
        s.to_string()
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
