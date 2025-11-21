#!/bin/bash

# Claude Code Stopフックスクリプト
# タスク完了時に詳細情報を含む通知を表示

# 標準入力からJSONデータを読み込む
input=$(cat)

# トランスクリプトパスを取得
transcript_path=$(echo "$input" | jq -r '.transcript_path // ""')

# ユーザープロンプトとアシスタントメッセージを抽出
user_prompt="リクエスト"
assistant_message="タスクが完了しました"

if [ -n "$transcript_path" ] && [ -f "$transcript_path" ]; then
    # トランスクリプトから最後のユーザーメッセージを抽出
    # 逆順で読み込んで、最初に見つかったuserメッセージを取得
    last_user_message=$(tac "$transcript_path" | \
                        jq -r 'select(.role=="user") |
                               if .content | type == "array" then
                                   .content[] | select(.type=="text") | .text
                               elif .content | type == "string" then
                                   .content
                               else
                                   empty
                               end' 2>/dev/null | \
                        grep -v "^$" | \
                        head -1 | \
                        sed 's/^[[:space:]]*//;s/[[:space:]]*$//' | \
                        cut -c1-80)

    # ユーザープロンプトが取得できた場合は使用
    if [ -n "$last_user_message" ]; then
        user_prompt="$last_user_message"
    fi

    # トランスクリプトから最後のアシスタントメッセージを抽出
    # 逆順で読み込んで、最初に見つかったassistantメッセージを取得
    last_assistant_message=$(tac "$transcript_path" | \
                             jq -r 'select(.role=="assistant") |
                                    if .content | type == "array" then
                                        .content[] | select(.type=="text") | .text
                                    elif .content | type == "string" then
                                        .content
                                    else
                                        empty
                                    end' 2>/dev/null | \
                             grep -v "^$" | \
                             head -1 | \
                             sed 's/^[[:space:]]*//;s/[[:space:]]*$//' | \
                             cut -c1-120)

    # メッセージが取得できた場合は使用
    if [ -n "$last_assistant_message" ]; then
        assistant_message="$last_assistant_message"
    fi
fi

# サブタイトルとメッセージを構築
subtitle="📝 $user_prompt"

# 通知を送信（terminal-notifierを使用）
# -activate で通知クリック時にターミナルに移動
terminal-notifier \
    -title "Claude Code - タスク完了" \
    -message "$assistant_message" \
    -subtitle "$subtitle" \
    -sound Funk \
    -activate com.apple.Terminal

# デバッグ用（必要に応じてコメント解除）
# echo "[$(date)] Prompt: $user_prompt | Response: $assistant_message" >> ~/.claude/task-complete.log
