# Claude Hooks - Rust実装

Claude Codeの通知フックシステムのRust実装です。

## 概要

このプロジェクトは、Claude Codeの各種フックイベントに対応したmacOS通知を送信するRustバイナリを提供します。Bashスクリプトから移行し、以下の改善を実現しています：

- **型安全性**: SerdeによるJSON入力の厳密な型チェック
- **パフォーマンス**: ネイティブバイナリによる高速実行
- **保守性**: 共通ロジックのライブラリ化
- **拡張性**: 複数の通知タイプへの柔軟な対応

## アーキテクチャ

### バイナリ構成

このプロジェクトは2つのバイナリを生成します：

1. **permission-notification**: `Notification`および`PermissionRequest`フック用
2. **task-complete-notification**: `Stop`フック用

### 主要コンポーネント

#### `src/lib.rs` - 共通ライブラリ

両バイナリで共有される機能を提供：

- **IDE/ターミナル検出**
  - `detect_ide_bundle_id()`: `~/.claude/ide/*.lock`から実行中のIDEを検出
  - `detect_terminal_bundle_id()`: 環境変数からターミナルを検出
  - `get_activation_bundle_id()`: IDE優先でBundle IDを取得

- **通知送信**
  - `send_notification()`: terminal-notifierを使用してmacOS通知を送信

- **ユーティリティ**
  - `get_dir_name()`: カレントディレクトリ名を取得
  - `get_relative_path()`: 相対パス変換
  - `extract_user_prompt()`: トランスクリプトからユーザープロンプト抽出
  - `extract_assistant_message()`: トランスクリプトからアシスタントメッセージ抽出

#### `src/bin/permission-notification.rs`

`Notification`および`PermissionRequest`フックで使用されるバイナリ。

**対応する通知タイプ:**

- `idle_prompt`: 60秒以上アイドル時の通知
- `permission_prompt`: ツール実行許可リクエスト（defaultMode時）
- その他: カスタム通知タイプ

**入力JSON構造:**

```rust
struct HookInput {
    session_id: String,
    cwd: String,
    tool_name: Option<String>,        // idle_promptでは不要
    tool_input: Option<Value>,        // idle_promptでは不要
    notification_type: Option<String>, // "idle_prompt", "permission_prompt"など
    message: Option<String>,           // カスタムメッセージ
}
```

**通知メッセージ生成ロジック:**

`build_tool_message()`関数で各ツールに応じた絵文字とメッセージを生成：

- `Bash`: 🔧 コマンド実行
- `Read`: 📖 ファイル読み込み
- `Write`: ✍️ ファイル作成
- `Edit`: ✏️ ファイル編集
- `Grep`: 🔍 コード検索
- `Glob`: 🔍 ファイル検索
- `Task`: 🤖 エージェント実行

#### `src/bin/task-complete-notification.rs`

`Stop`フックで使用されるバイナリ。セッション終了時にタスク完了通知を送信。

**入力JSON構造:**

```rust
struct StopHookInput {
    session_id: String,
    transcript_path: Option<String>,
    cwd: String,
}
```

**動作:**

1. トランスクリプトファイルを解析
2. 最後のユーザープロンプトを抽出（サブタイトルに使用）
3. 最後のアシスタントメッセージを抽出（本文に使用）
4. 通知を送信（サウンド: "Funk"）

## ビルド

### 開発ビルド

```bash
cargo build
```

### リリースビルド

```bash
cargo build --release
```

ビルドされたバイナリは`target/release/`に生成されます。

### インストール

```bash
# binディレクトリにコピー
cp target/release/permission-notification ../bin/
cp target/release/task-complete-notification ../bin/

# 実行権限を付与
chmod +x ../bin/permission-notification
chmod +x ../bin/task-complete-notification
```

## テスト

### 手動テスト - permission-notification

```bash
# idle_prompt通知のテスト
echo '{"session_id":"test","cwd":"'$(pwd)'","notification_type":"idle_prompt","message":"テストメッセージ"}' | \
  ./target/release/permission-notification

# permission_prompt通知のテスト
echo '{"session_id":"test","cwd":"'$(pwd)'","notification_type":"permission_prompt","tool_name":"Bash","tool_input":{"command":"ls -la","description":"ファイル一覧表示"}}' | \
  ./target/release/permission-notification
```

### 手動テスト - task-complete-notification

```bash
# トランスクリプトなしでテスト
echo '{"session_id":"test","cwd":"'$(pwd)'"}' | \
  ./target/release/task-complete-notification

# トランスクリプトありでテスト（実際のトランスクリプトパスを指定）
echo '{"session_id":"test","cwd":"'$(pwd)'","transcript_path":"/path/to/transcript.jsonl"}' | \
  ./target/release/task-complete-notification
```

## カスタマイズ

### 新しいツールタイプの追加

`src/bin/permission-notification.rs`の`build_tool_message()`関数に新しいケースを追加：

```rust
"NewTool" => {
    let subtitle = "🆕 新しいツール".to_string();
    let message = // ツール固有のメッセージ生成
    (subtitle, message)
}
```

### 通知サウンドの変更

- permission-notification: `src/bin/permission-notification.rs:53` - "Glass"
- task-complete-notification: `src/bin/task-complete-notification.rs:42` - "Funk"

macOSのサウンド一覧:
```bash
ls /System/Library/Sounds/
```

### IDE/ターミナル検出の拡張

`src/lib.rs`の検出関数を修正：

- `detect_terminal_bundle_id()`: 環境変数ベースの検出
- `detect_ide_bundle_id()`: lockファイルベースの検出

## デバッグ

### ログ出力

`task-complete-notification`は`~/.claude/task-complete.log`にログを出力します：

```bash
tail -f ~/.claude/task-complete.log
```

### エラーハンドリング

両バイナリは標準的なRust `io::Result`を使用：

- JSON解析エラー: `InvalidData`エラーとして返される
- ファイルI/Oエラー: そのまま伝播
- 通知送信エラー: `terminal-notifier`の終了コードで判定

## 依存関係

```toml
[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
chrono = "0.4"
```

- **serde**: JSON入力のデシリアライズ
- **serde_json**: JSON値の動的処理
- **chrono**: タイムスタンプ生成（ログ用）

## パフォーマンス

- 起動時間: ~10ms（リリースビルド）
- メモリ使用量: ~1MB
- バイナリサイズ: ~600-800KB（リリースビルド、strip済み）

Bashスクリプト版と比較して約5-10倍高速です。

## トラブルシューティング

### バイナリが実行されない

1. 実行権限を確認:
```bash
ls -la ~/.claude/bin/permission-notification
```

2. 依存関係を確認:
```bash
otool -L ~/.claude/bin/permission-notification
```

### IDE検出が動作しない

1. lockファイルを確認:
```bash
ls -la ~/.claude/ide/
cat ~/.claude/ide/*.lock
```

2. プロセスが実行中か確認:
```bash
ps -p <PID>
```

### 通知が表示されない

1. terminal-notifierが利用可能か確認:
```bash
which terminal-notifier
terminal-notifier -message "Test" -title "Test"
```

2. Bundle IDが正しいか確認:
```bash
# IDE検出のテスト
echo '{"session_id":"test","cwd":"'$(pwd)'","notification_type":"idle_prompt","message":"test"}' | \
  ./target/release/permission-notification
```

## 今後の拡張案

- [ ] 通知の優先度設定
- [ ] 通知のグルーピング
- [ ] カスタム通知テンプレート
- [ ] より詳細なエラーログ
- [ ] ユニットテスト/インテグレーションテストの追加

## ライセンス

MIT License
