#!/bin/bash

# Claude Code IDE検出機能
# IDEモードで実行中のClaude CodeのIDEを検出する

# lockファイルからアクティブなIDEのBundle IDを検出
detect_ide_bundle_id() {
    local lock_dir="$HOME/.claude/ide"

    # lockディレクトリの存在確認
    if [ ! -d "$lock_dir" ]; then
        return 1
    fi

    # 最新の.lockファイルを取得（更新日時順）
    local latest_lock=$(ls -t "$lock_dir"/*.lock 2>/dev/null | head -1)

    if [ -z "$latest_lock" ]; then
        return 1
    fi

    # JSONからPIDを抽出
    local pid=$(jq -r '.pid // empty' "$latest_lock" 2>/dev/null)

    if [ -z "$pid" ]; then
        return 1
    fi

    # プロセスが実行中か確認
    if ! ps -p "$pid" > /dev/null 2>&1; then
        return 1
    fi

    # アプリケーションパスを取得
    local app_path=$(ps -p "$pid" -o comm= | sed 's|/Contents/MacOS/.*||' | head -1)

    if [ -z "$app_path" ] || [ ! -d "$app_path" ]; then
        return 1
    fi

    # Bundle IDを取得
    local bundle_id=$(mdls -name kMDItemCFBundleIdentifier "$app_path" 2>/dev/null | cut -d'"' -f2)

    if [ -n "$bundle_id" ]; then
        echo "$bundle_id"
        return 0
    fi

    return 1
}

# IDE名を取得（ログ用）
get_ide_name_from_bundle_id() {
    local bundle_id="$1"

    case "$bundle_id" in
        "com.microsoft.VSCode")
            echo "Visual Studio Code"
            ;;
        "com.microsoft.VSCodeInsiders")
            echo "VSCode Insiders"
            ;;
        com.todesktop.*)
            echo "Cursor"
            ;;
        "com.jetbrains.intellij.ce")
            echo "IntelliJ IDEA CE"
            ;;
        "com.jetbrains.intellij")
            echo "IntelliJ IDEA"
            ;;
        "com.jetbrains.fleet")
            echo "Fleet"
            ;;
        "com.apple.dt.Xcode")
            echo "Xcode"
            ;;
        *)
            echo "Unknown IDE ($bundle_id)"
            ;;
    esac
}
