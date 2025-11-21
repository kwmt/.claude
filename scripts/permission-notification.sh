#!/bin/bash

# Claude Code Notificationフックスクリプト
# 確認待ち時に詳細情報を含む通知を表示

# 標準入力からJSONデータを読み込む
input=$(cat)

# ツール名を取得
tool_name=$(echo "$input" | jq -r '.tool_name // "Unknown"')

# 現在のディレクトリを取得（相対パス表示用）
current_dir=$(pwd)

# ツール固有の情報を抽出
detail=""
subtitle=""

case "$tool_name" in
  "Bash")
    # Bashコマンドの説明またはコマンド本体を取得
    description=$(echo "$input" | jq -r '.tool_input.description // ""')
    command=$(echo "$input" | jq -r '.tool_input.command // ""')

    if [ -n "$description" ]; then
        subtitle="🔧 コマンド実行"
        detail="$description"
    elif [ -n "$command" ]; then
        subtitle="🔧 コマンド実行"
        detail="$command"
    fi

    # 長すぎる場合は省略
    detail=$(echo "$detail" | cut -c1-150)
    ;;

  "Read")
    # ファイルパスを取得して相対パスに変換
    file_path=$(echo "$input" | jq -r '.tool_input.file_path // ""')
    if [ -n "$file_path" ]; then
        # 相対パスに変換を試みる
        rel_path="${file_path#$current_dir/}"
        if [ "$rel_path" = "$file_path" ]; then
            # 変換できない場合はbasenameを使用
            rel_path=$(basename "$file_path")
        fi
        subtitle="📖 ファイル読み込み"
        detail="$rel_path"
    fi
    ;;

  "Write")
    # ファイルパスを取得して相対パスに変換
    file_path=$(echo "$input" | jq -r '.tool_input.file_path // ""')
    if [ -n "$file_path" ]; then
        # 相対パスに変換を試みる
        rel_path="${file_path#$current_dir/}"
        if [ "$rel_path" = "$file_path" ]; then
            # 変換できない場合はbasenameを使用
            rel_path=$(basename "$file_path")
        fi
        subtitle="✍️ ファイル作成"
        detail="$rel_path"
    fi
    ;;

  "Edit")
    # ファイルパスを取得して相対パスに変換
    file_path=$(echo "$input" | jq -r '.tool_input.file_path // ""')
    if [ -n "$file_path" ]; then
        # 相対パスに変換を試みる
        rel_path="${file_path#$current_dir/}"
        if [ "$rel_path" = "$file_path" ]; then
            # 変換できない場合はbasenameを使用
            rel_path=$(basename "$file_path")
        fi
        subtitle="✏️ ファイル編集"
        detail="$rel_path"
    fi
    ;;

  "Grep")
    # 検索パターンを取得
    pattern=$(echo "$input" | jq -r '.tool_input.pattern // ""')
    if [ -n "$pattern" ]; then
        subtitle="🔍 コード検索"
        detail="パターン: $pattern"
    fi
    ;;

  "Glob")
    # Globパターンを取得
    pattern=$(echo "$input" | jq -r '.tool_input.pattern // ""')
    if [ -n "$pattern" ]; then
        subtitle="🔍 ファイル検索"
        detail="パターン: $pattern"
    fi
    ;;

  "Task")
    # サブエージェントタイプを取得
    subagent=$(echo "$input" | jq -r '.tool_input.subagent_type // ""')
    if [ -n "$subagent" ]; then
        subtitle="🤖 エージェント実行"
        detail="タイプ: $subagent"
    fi
    ;;

  *)
    # その他のツール
    subtitle="🔧 ツール実行"
    detail="$tool_name"
    ;;
esac

# 通知メッセージを構築
if [ -n "$detail" ]; then
    message="$detail"
else
    message="${tool_name}の実行を確認しています"
fi

# 通知を送信（terminal-notifierを使用）
# -activate で通知クリック時にターミナルに移動
terminal-notifier \
    -title "Claude Code - 確認待ち" \
    -message "$message" \
    -subtitle "$subtitle" \
    -sound Glass \
    -activate com.apple.Terminal

# デバッグ用（必要に応じてコメント解除）
# echo "[$(date)] Tool: $tool_name | Detail: $detail" >> ~/.claude/notification.log
