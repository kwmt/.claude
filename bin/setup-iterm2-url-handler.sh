#!/bin/bash
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
APP_PATH="$SCRIPT_DIR/iTerm2Switch.app"
APPLESCRIPT_SRC="$SCRIPT_DIR/iTerm2Switch.applescript"

# 既存アプリを削除
rm -rf "$APP_PATH"

# 1. AppleScriptをアプリにコンパイル（stay-open）
osacompile -s -o "$APP_PATH" "$APPLESCRIPT_SRC"

# 2. Info.plistにCFBundleIdentifier + URLスキームを登録
PLIST="$APP_PATH/Contents/Info.plist"
/usr/libexec/PlistBuddy -c "Add :CFBundleIdentifier string 'com.claude.iTerm2Switch'" "$PLIST"
/usr/libexec/PlistBuddy -c "Add :CFBundleURLTypes array" "$PLIST"
/usr/libexec/PlistBuddy -c "Add :CFBundleURLTypes:0 dict" "$PLIST"
/usr/libexec/PlistBuddy -c "Add :CFBundleURLTypes:0:CFBundleURLName string 'Claude iTerm2 Switch'" "$PLIST"
/usr/libexec/PlistBuddy -c "Add :CFBundleURLTypes:0:CFBundleURLSchemes array" "$PLIST"
/usr/libexec/PlistBuddy -c "Add :CFBundleURLTypes:0:CFBundleURLSchemes:0 string 'x-claude-iterm'" "$PLIST"

# 3. 再署名（Info.plist変更後に必要）
codesign --force --sign - "$APP_PATH"

# 4. Quarantine属性を削除（Gatekeeper対策）
xattr -dr com.apple.quarantine "$APP_PATH" 2>/dev/null

# 5. LaunchServicesにURLスキームを登録
/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -f "$APP_PATH"

echo "iTerm2Switch.app created and registered for x-claude-iterm:// URL scheme"

# 6. LaunchAgentを生成・登録（ログイン時に自動で再登録されるようにする）
# plistが既に存在する場合はスキップ（launchdからの実行時に無限ループを防止）
LAUNCHAGENT_DIR="$HOME/Library/LaunchAgents"
LAUNCHAGENT_PLIST="$LAUNCHAGENT_DIR/com.claude.iterm2-url-handler.plist"

if [ ! -f "$LAUNCHAGENT_PLIST" ]; then
    mkdir -p "$LAUNCHAGENT_DIR"

    cat > "$LAUNCHAGENT_PLIST" <<PLIST_EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.claude.iterm2-url-handler</string>
    <key>ProgramArguments</key>
    <array>
        <string>/bin/bash</string>
        <string>${SCRIPT_DIR}/setup-iterm2-url-handler.sh</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>StandardOutPath</key>
    <string>/tmp/claude-iterm2-setup.log</string>
    <key>StandardErrorPath</key>
    <string>/tmp/claude-iterm2-setup.log</string>
</dict>
</plist>
PLIST_EOF

    launchctl bootstrap "gui/$(id -u)" "$LAUNCHAGENT_PLIST" 2>/dev/null
    echo ""
    echo "LaunchAgentを登録しました（macOS起動時に自動で再実行されます）"
else
    echo ""
    echo "LaunchAgentは登録済みです"
fi

echo ""
echo "初回のみ: Automation権限の許可が必要です。"
echo "「システム設定 > プライバシーとセキュリティ > オートメーション」で"
echo "iTerm2Switch.app → iTerm2 を許可してください。"
