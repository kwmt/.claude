#!/bin/bash

# Claude Code Stopフックスクリプト
# タスク完了時に詳細情報を含む通知を表示

# 標準入力からJSONデータを読み込む
input=$(cat)

# セッションIDを取得（先頭8文字のみ）
session_id=$(echo "$input" | jq -r '.session_id // "unknown"' | cut -c1-8)

# トランスクリプトパスを取得
transcript_path=$(echo "$input" | jq -r '.transcript_path // ""')

# タスク情報を抽出
task_info="タスクが完了しました"

if [ -n "$transcript_path" ] && [ -f "$transcript_path" ]; then
    # トランスクリプトから最後のアシスタントメッセージを抽出
    # 最後の数行を取得し、役割がassistantのメッセージの内容を抽出
    last_message=$(tail -100 "$transcript_path" | \
                   jq -r 'select(.role=="assistant") |
                          if .content | type == "array" then
                              .content[] | select(.type=="text") | .text
                          elif .content | type == "string" then
                              .content
                          else
                              empty
                          end' 2>/dev/null | \
                   grep -v "^$" | \
                   tail -1 | \
                   sed 's/^[[:space:]]*//;s/[[:space:]]*$//' | \
                   cut -c1-100)

    # メッセージが取得できた場合は使用
    if [ -n "$last_message" ]; then
        task_info="$last_message"
    fi
fi

# 通知を送信（terminal-notifierを使用）
terminal-notifier \
    -title "Claude Code" \
    -message "$task_info" \
    -subtitle "Session: $session_id" \
    -sound Funk

# デバッグ用（必要に応じてコメント解除）
# echo "[$(date)] Session: $session_id | Task: $task_info" >> ~/.claude/task-complete.log
