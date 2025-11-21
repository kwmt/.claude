#!/bin/bash

# Claude Code 共通関数
# 実行中のターミナルアプリケーションを検出する

# ターミナルアプリケーションのBundle IDを検出する関数
detect_terminal_bundle_id() {
    local bundle_id=""

    # 1. TERM_PROGRAM環境変数で検出（最も信頼性が高い）
    if [ -n "$TERM_PROGRAM" ]; then
        case "$TERM_PROGRAM" in
            "iTerm.app")
                bundle_id="com.googlecode.iterm2"
                ;;
            "Apple_Terminal")
                bundle_id="com.apple.Terminal"
                ;;
            "WarpTerminal")
                bundle_id="dev.warp.Warp-Stable"
                ;;
            "Hyper")
                bundle_id="co.zeit.hyper"
                ;;
        esac
    fi

    # 2. ターミナル固有の環境変数で検出
    if [ -z "$bundle_id" ]; then
        if [ -n "$ITERM_SESSION_ID" ]; then
            bundle_id="com.googlecode.iterm2"
        elif [ -n "$ALACRITTY_SOCKET" ]; then
            bundle_id="io.alacritty.Alacritty"
        elif [ -n "$KITTY_WINDOW_ID" ]; then
            bundle_id="net.kovidgoyal.kitty"
        elif [ -n "$WARP_IS_LOCAL_SHELL_SESSION" ]; then
            bundle_id="dev.warp.Warp-Stable"
        fi
    fi

    # 3. LC_TERMINAL環境変数で検出
    if [ -z "$bundle_id" ] && [ -n "$LC_TERMINAL" ]; then
        case "$LC_TERMINAL" in
            "iTerm2")
                bundle_id="com.googlecode.iterm2"
                ;;
            "Terminal")
                bundle_id="com.apple.Terminal"
                ;;
        esac
    fi

    # 4. TERM環境変数で推測（信頼性は低い）
    if [ -z "$bundle_id" ] && [ -n "$TERM" ]; then
        case "$TERM" in
            "xterm-kitty")
                bundle_id="net.kovidgoyal.kitty"
                ;;
            "alacritty")
                bundle_id="io.alacritty.Alacritty"
                ;;
        esac
    fi

    # 5. プロセス情報から検出（最終手段）
    if [ -z "$bundle_id" ]; then
        local parent_chain=$(ps -o command= -p $PPID 2>/dev/null)

        if echo "$parent_chain" | grep -q "iTerm"; then
            bundle_id="com.googlecode.iterm2"
        elif echo "$parent_chain" | grep -q "Terminal.app"; then
            bundle_id="com.apple.Terminal"
        elif echo "$parent_chain" | grep -q "Warp"; then
            bundle_id="dev.warp.Warp-Stable"
        elif echo "$parent_chain" | grep -q "Alacritty"; then
            bundle_id="io.alacritty.Alacritty"
        elif echo "$parent_chain" | grep -q "kitty"; then
            bundle_id="net.kovidgoyal.kitty"
        fi
    fi

    # 6. フォールバック: Terminal.appをデフォルトとする
    if [ -z "$bundle_id" ]; then
        bundle_id="com.apple.Terminal"
    fi

    echo "$bundle_id"
}

# ターミナルのフレンドリー名を取得する関数（ログ用）
get_terminal_name() {
    local bundle_id="$1"

    case "$bundle_id" in
        "com.googlecode.iterm2")
            echo "iTerm2"
            ;;
        "com.apple.Terminal")
            echo "Terminal"
            ;;
        "dev.warp.Warp-Stable")
            echo "Warp"
            ;;
        "io.alacritty.Alacritty")
            echo "Alacritty"
            ;;
        "net.kovidgoyal.kitty")
            echo "Kitty"
            ;;
        "co.zeit.hyper")
            echo "Hyper"
            ;;
        *)
            echo "Unknown ($bundle_id)"
            ;;
    esac
}
