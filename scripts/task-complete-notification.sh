#!/bin/bash

# Claude Code Stopフックスクリプト
# タスク完了時に詳細情報を含む通知を表示

# 共通関数を読み込む
source ~/.claude/scripts/detect-terminal.sh

# 標準入力からJSONデータを読み込む
input=$(cat)

# トランスクリプトパスを取得
transcript_path=$(echo "$input" | jq -r '.transcript_path // ""')

# カレントディレクトリを取得（優先順位: cwd → CLAUDE_PROJECT_DIR → pwd）
cwd=$(echo "$input" | jq -r '.cwd // ""')
current_dir="${cwd:-${CLAUDE_PROJECT_DIR:-$(pwd)}}"
dir_name=$(basename "$current_dir")

# ターミナルアプリのBundle IDを検出
TERMINAL_BUNDLE_ID=$(detect_terminal_bundle_id)

# デバッグログディレクトリ
log_file="$HOME/.claude/task-complete.log"

# ユーザープロンプトとアシスタントメッセージを抽出
user_prompt="リクエスト"
assistant_message="タスクが完了しました"

if [ -n "$transcript_path" ] && [ -f "$transcript_path" ]; then
    # ユーザーの実際のプロンプトを抽出（メタメッセージやコマンド関連を除外）
    last_user_message=$(jq -s 'reverse | .[] |
                               select(.type == "user") |
                               select((.isMeta // false) == false) |
                               if .message.content | type == "string" then
                                 .message.content
                               elif .message.content | type == "array" then
                                 .message.content[] | select(.type == "text") | .text
                               else
                                 empty
                               end' "$transcript_path" 2>/dev/null | \
                        grep -v "<command-name>" | \
                        grep -v "<command-message>" | \
                        grep -v "<command-args>" | \
                        grep -v "<local-command-stdout>" | \
                        grep -v "^Caveat:" | \
                        grep -v "^$" | \
                        head -1 | \
                        sed 's/^[[:space:]]*//;s/[[:space:]]*$//' | \
                        head -c 100)

    # ユーザープロンプトが取得できた場合は使用
    if [ -n "$last_user_message" ]; then
        user_prompt="$last_user_message"
        # 長すぎる場合は省略記号を追加
        if [ ${#last_user_message} -ge 100 ]; then
            user_prompt="${user_prompt}..."
        fi
    fi

    # アシスタントの最後のメッセージを抽出
    last_assistant_message=$(jq -s 'reverse | .[] |
                                    select(.type == "assistant") |
                                    if .message.content | type == "string" then
                                      .message.content
                                    elif .message.content | type == "array" then
                                      .message.content[] | select(.type == "text") | .text
                                    else
                                      empty
                                    end' "$transcript_path" 2>/dev/null | \
                             head -1 | \
                             sed 's/^[[:space:]]*//;s/[[:space:]]*$//' | \
                             grep -v '^$' | \
                             head -c 150)

    # メッセージが取得できた場合は使用
    if [ -n "$last_assistant_message" ]; then
        assistant_message="$last_assistant_message"
        # 長すぎる場合は省略記号を追加
        if [ ${#last_assistant_message} -ge 150 ]; then
            assistant_message="${assistant_message}..."
        fi
    fi

    # デバッグログ出力
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] Transcript: $transcript_path" >> "$log_file"
    echo "  User Prompt: $user_prompt" >> "$log_file"
    echo "  Assistant: $assistant_message" >> "$log_file"
    echo "" >> "$log_file"
fi

# サブタイトルとメッセージを構築
subtitle="📝 $user_prompt"

# 通知を送信（terminal-notifierを使用）
# -activate で通知クリック時に実行中のターミナルに移動
terminal-notifier \
    -title "Claude Code - タスク完了 ($dir_name)" \
    -message "$assistant_message" \
    -subtitle "$subtitle" \
    -sound Funk \
    -activate "$TERMINAL_BUNDLE_ID"
