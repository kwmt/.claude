#!/bin/bash

# Claude Code Notificationフックスクリプト
# 確認待ち時に詳細情報を含む通知を表示

# 標準入力からJSONデータを読み込む
input=$(cat)

# セッションIDを取得（先頭8文字のみ）
session_id=$(echo "$input" | jq -r '.session_id // "unknown"' | cut -c1-8)

# ツール名を取得
tool_name=$(echo "$input" | jq -r '.tool_name // "Unknown"')

# ツール固有の情報を抽出
detail=""
case "$tool_name" in
  "Bash")
    # Bashコマンドの説明またはコマンド本体を取得
    description=$(echo "$input" | jq -r '.tool_input.description // ""')
    command=$(echo "$input" | jq -r '.tool_input.command // ""')

    if [ -n "$description" ]; then
        detail="$description"
    elif [ -n "$command" ]; then
        detail="$command"
    fi

    # 長すぎる場合は省略
    detail=$(echo "$detail" | cut -c1-100)
    ;;

  "Read")
    # ファイルパスを取得
    file_path=$(echo "$input" | jq -r '.tool_input.file_path // ""')
    if [ -n "$file_path" ]; then
        detail="読み込み: $(basename "$file_path")"
    fi
    ;;

  "Write")
    # ファイルパスを取得
    file_path=$(echo "$input" | jq -r '.tool_input.file_path // ""')
    if [ -n "$file_path" ]; then
        detail="書き込み: $(basename "$file_path")"
    fi
    ;;

  "Edit")
    # ファイルパスを取得
    file_path=$(echo "$input" | jq -r '.tool_input.file_path // ""')
    if [ -n "$file_path" ]; then
        detail="編集: $(basename "$file_path")"
    fi
    ;;

  *)
    # その他のツール
    detail="${tool_name}の実行"
    ;;
esac

# 通知メッセージを構築
if [ -n "$detail" ]; then
    message="$detail"
else
    message="${tool_name}の実行を確認しています"
fi

# 通知を送信（terminal-notifierを使用）
terminal-notifier \
    -title "Claude Code" \
    -message "$message" \
    -subtitle "確認待ち | Session: $session_id" \
    -sound Glass

# デバッグ用（必要に応じてコメント解除）
# echo "[$(date)] Session: $session_id | Tool: $tool_name | Detail: $detail" >> ~/.claude/notification.log
