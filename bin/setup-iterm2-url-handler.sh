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
echo ""
echo "初回のみ: Automation権限の許可が必要です。"
echo "「システム設定 > プライバシーとセキュリティ > オートメーション」で"
echo "iTerm2Switch.app → iTerm2 を許可してください。"
