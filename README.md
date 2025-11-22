# Claude Code カスタマイズ設定

このディレクトリには、Claude Codeの動作をカスタマイズする設定とスクリプトが含まれています。

## 主な機能

### 通知システム（Rust実装）

Claude Codeの各種イベントでmacOS通知センターに通知を表示します。

#### 通知の種類

1. **PermissionRequest通知** - ツール実行確認時
   - CLIで「Would you like to proceed?」が表示されるタイミングで通知
   - ツール名と操作内容を表示
   - 例: 「🔧 コマンド実行 - git status」

2. **アイドル通知** - 60秒以上入力待機時
   - 長時間入力がない場合に通知
   - 例: 「⏱️ アイドル状態 - 入力を待っています」

3. **タスク完了通知** - セッション終了時
   - ユーザーのリクエストと完了内容を表示
   - 例: 「📝 バグ修正 - 修正が完了しました」

#### 通知の特徴

- **IDE/ターミナル自動検出**: VSCode、Cursor、iTerm2などを自動認識し、通知タップで該当アプリをアクティブ化
- **日本語ローカライズ**: 全ての通知メッセージが日本語
- **ツール別アイコン**: Bash、Read、Write、Edit、Grep、Globなど各ツールに専用の絵文字アイコン

### 権限制御

`settings.json`で特定のコマンドを拒否リストに登録できます。

```json
"deny": [
  "Bash(git config:*)",
  "Bash(brew install:*)",
  "Bash(chmod 777:*)",
  "Bash(rm -rf /*)",
  "Bash(gh repo delete:*)"
]
```

## ディレクトリ構造

```
.claude/
├── settings.json              # Claude Code設定ファイル
├── README.md                  # このファイル
├── bin/                       # 実行可能バイナリ
│   ├── permission-notification     # PermissionRequest/Notification用
│   └── task-complete-notification  # Stop用
├── scripts/                   # シェルスクリプト
│   └── deny-check.sh              # PreToolUse用（コマンド拒否チェック）
├── scripts-rust/              # Rustソースコード
│   ├── src/
│   │   ├── lib.rs            # 共通ライブラリ
│   │   └── bin/              # バイナリソース
│   ├── Cargo.toml
│   └── README.md             # 開発者向けドキュメント
└── statusline.js             # ステータスライン表示
```

## セットアップ

### 必要な環境

- macOS
- [terminal-notifier](https://github.com/julienXX/terminal-notifier)（通知表示用）
- Rust（ビルド時のみ）

### インストール

1. terminal-notifierのインストール:
```bash
brew install terminal-notifier
```

2. スクリプトのビルド（既にバイナリが含まれている場合は不要）:
```bash
cd ~/.claude/scripts-rust
cargo build --release
cp target/release/permission-notification ../bin/
cp target/release/task-complete-notification ../bin/
```

3. settings.jsonの確認:

`~/.claude/settings.json`に以下の設定が含まれていることを確認してください:

```json
{
  "permissions": {
    "deny": [...],
    "defaultMode": "bypassPermissions"
  },
  "hooks": {
    "PreToolUse": [...],
    "Notification": [
      {
        "matcher": "",
        "hooks": [
          {
            "type": "command",
            "command": "~/.claude/bin/permission-notification"
          }
        ]
      }
    ],
    "PermissionRequest": [
      {
        "matcher": "",
        "hooks": [
          {
            "type": "command",
            "command": "~/.claude/bin/permission-notification"
          }
        ]
      }
    ],
    "Stop": [
      {
        "matcher": "",
        "hooks": [
          {
            "type": "command",
            "command": "~/.claude/bin/task-complete-notification"
          }
        ]
      }
    ]
  }
}
```

## 使い方

設定後は自動的に動作します：

1. **ツール実行時**: CLIで確認ダイアログが表示されると同時に通知が届きます
2. **アイドル時**: 60秒以上入力がないと「入力待ち」通知が表示されます
3. **タスク完了時**: セッション終了時に完了通知が表示されます

通知をタップすると、実行中のIDE/ターミナルがアクティブになります。

## カスタマイズ

通知の挙動をカスタマイズする場合は、`scripts-rust/README.md`を参照してください。

## トラブルシューティング

### 通知が表示されない

1. terminal-notifierがインストールされているか確認:
```bash
which terminal-notifier
```

2. バイナリに実行権限があるか確認:
```bash
ls -la ~/.claude/bin/permission-notification
ls -la ~/.claude/bin/task-complete-notification
```

3. 手動でテスト実行:
```bash
echo '{"session_id":"test","cwd":"'$(pwd)'","notification_type":"idle_prompt","message":"テスト"}' | ~/.claude/bin/permission-notification
```

### 通知タップでアプリがアクティブにならない

IDE/ターミナルの自動検出は以下をサポートしています:
- VSCode
- Cursor
- iTerm2
- Apple Terminal
- Warp
- Kitty
- Alacritty
- Hyper

サポート外のアプリの場合、Apple Terminalがデフォルトで使用されます。

## ライセンス

このカスタマイズはMITライセンスです。
