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

このプロジェクトは6つのバイナリを生成します：

1. **permission-notification**: `Notification`および`PermissionRequest`フック用
2. **task-complete-notification**: `Stop`フック用
3. **user-prompt-slack**: `UserPromptSubmit`フック用（Slack通知専用）
4. **askuser-answer-slack**: `PostToolUse` (AskUserQuestion) フック用（Slack通知専用）
5. **askuser-question-slack**: `PostToolUse` (AskUserQuestion) フック用（Slack通知専用）
6. **exitplanmode-slack**: `PostToolUse` (ExitPlanMode) フック用（Slack通知専用）

### 主要コンポーネント

#### `src/lib.rs` - 共通ライブラリ

両バイナリで共有される機能を提供：

- **IDE/ターミナル検出**
  - `detect_ide_bundle_id()`: `~/.claude/ide/*.lock`から実行中のIDEを検出
  - `detect_terminal_bundle_id()`: 環境変数からターミナルを検出
  - `get_activation_bundle_id()`: IDE優先でBundle IDを取得

- **通知送信**
  - `send_notification()`: terminal-notifierを使用してmacOS通知を送信
  - `post_to_slack_rich()`: Slack Block Kitを使用したリッチフォーマット通知

- **ユーティリティ**
  - `get_dir_name()`: カレントディレクトリ名を取得
  - `get_relative_path()`: 相対パス変換
  - `extract_user_prompt()`: トランスクリプトからユーザープロンプト抽出
  - `extract_assistant_message()`: トランスクリプトからアシスタントメッセージ抽出

- **コンテンツ処理**
  - `truncate_content()`: コンテンツを2800文字で切り詰め
  - `extract_questions_with_options()`: AskUserQuestionのtool_inputから質問とオプションを抽出

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
4. macOS通知を送信（サウンド: "Funk"）
5. Slack通知を送信（環境変数が設定されている場合）

#### `src/bin/user-prompt-slack.rs`

`UserPromptSubmit`フックで使用されるバイナリ。ユーザーがプロンプトを送信したタイミングでSlack通知を送信。

**入力JSON構造:**

```rust
struct UserPromptSubmitInput {
    session_id: String,
    transcript_path: Option<String>,
    cwd: String,
    permission_mode: String,
    hook_event_name: String,
    prompt: String,
}
```

**動作:**

1. ユーザープロンプトを取得（200文字で切り詰め）
2. Slack Block Kit形式でリッチな通知を送信
   - ディレクトリ名
   - Permission Mode（bypassPermissions/default等）
   - プロンプト内容

#### `src/bin/askuser-answer-slack.rs`

`PostToolUse` (AskUserQuestion) フックで使用されるバイナリ。ユーザーが質問に回答したタイミングでSlack通知を送信。

**入力JSON構造:**

```rust
struct PostToolUseInput {
    session_id: String,
    cwd: String,
    tool_name: String,
    tool_input: Value,
    tool_response: Value,
}
```

**動作:**

1. tool_responseからユーザーの回答を抽出
2. 回答が空でない場合のみSlack通知を送信
3. 質問内容と回答をリッチフォーマットで表示

#### `src/bin/askuser-question-slack.rs`

`PostToolUse` (AskUserQuestion) フックで使用されるバイナリ。Claudeが質問を投げかけたタイミングでSlack通知を送信。

**入力JSON構造:**

```rust
struct PostToolUseInput {
    session_id: String,
    cwd: String,
    tool_name: String,
    tool_input: Value,
    tool_response: Value,
}
```

**動作:**

1. tool_inputから質問内容とオプションを抽出
2. Slack Block Kit形式で質問とオプション一覧を送信
3. 各オプションのラベルと説明を表示

#### `src/bin/exitplanmode-slack.rs`

`PostToolUse` (ExitPlanMode) フックで使用されるバイナリ。プランモード終了時にSlack通知を送信。

**入力JSON構造:**

```rust
struct PostToolUseInput {
    session_id: String,
    cwd: String,
    tool_name: String,
    tool_input: Value,
    tool_response: Value,
}
```

**動作:**

1. `~/.claude/plans/`から最新の.mdファイルを検索
2. プランファイルの内容を読み込み
3. 2800文字で切り詰めてSlack通知を送信

## Slack通知機能

### 概要

全ての通知バイナリは、macOS通知に加えてSlackへの通知をサポートしています。Slack Block Kitを使用したリッチフォーマットで、見やすく構造化されたメッセージを送信します。

### セットアップ

#### 1. Slack Incoming Webhookの作成

1. Slackワークスペースの[Incoming Webhooks](https://api.slack.com/messaging/webhooks)ページにアクセス
2. "Create New Webhook"をクリック
3. 通知を送信するチャンネルを選択
4. Webhook URLをコピー（例: `https://hooks.slack.com/services/T00000000/B00000000/XXXXXXXXXXXXXXXXXXXX`）

#### 2. 環境変数の設定

Webhook URLを環境変数に設定します。Codexの`SLACK_WEBHOOK_URL`と競合しないよう、専用の環境変数名を使用します。

```bash
# ~/.zshrc または ~/.bashrc に追加
export CLAUDE_CODE_SLACK_WEBHOOK_URL="https://hooks.slack.com/services/YOUR/WEBHOOK/URL"
```

設定後、シェルを再起動するか、以下を実行：

```bash
source ~/.zshrc  # または source ~/.bashrc
```

#### 3. Claude Codeの再起動

settings.jsonの変更を反映するため、Claude Codeを再起動してください。

### 通知内容

各バイナリは以下の情報をSlackに送信します：

#### user-prompt-slack（プロンプト送信時）
- **タイトル**: 🤔 New Claude Prompt
- **フィールド**:
  - Directory: 作業ディレクトリ名
  - Permission Mode: 現在のPermissionモード
  - Prompt: ユーザーが入力したプロンプト（200文字まで）

#### task-complete-notification（タスク完了時）
- **タイトル**: ✅ Claude Code - Task Complete
- **フィールド**:
  - Directory: 作業ディレクトリ名
  - User Prompt: ユーザーのリクエスト内容
  - Assistant Response: Claudeの応答メッセージ

#### permission-notification（待機状態/権限リクエスト時）
- **タイトル**:
  - ⏱️ Claude Code - Idle（アイドル時）
  - 🔔 Claude Code - Permission Request（権限リクエスト時）
  - 📢 Claude Code - Notification（その他）
- **フィールド**:
  - Directory: 作業ディレクトリ名
  - Type: 通知タイプ（🔧 コマンド実行、📖 ファイル読み込み等）
  - Message: 詳細メッセージ

#### askuser-question-slack（質問時）
- **タイトル**: ❓ Claude Question
- **フィールド**:
  - Directory: 作業ディレクトリ名
  - Question: Claudeからの質問内容
  - Options: 選択可能なオプション一覧（ラベルと説明）

#### askuser-answer-slack（回答時）
- **タイトル**: 💬 User Answer
- **フィールド**:
  - Directory: 作業ディレクトリ名
  - Question: 元の質問内容
  - Answer: ユーザーの回答

#### exitplanmode-slack（プラン完了時）
- **タイトル**: 📋 Plan Ready
- **フィールド**:
  - Directory: 作業ディレクトリ名
  - Plan: プランファイルの内容（2800文字まで）

### フェイルセーフ設計

- Slack通知の失敗は既存のmacOS通知に影響しません
- 環境変数が未設定の場合、Slack通知は静かにスキップされます
- エラーは標準エラー出力に記録されますが、プログラムは正常終了します

### トラブルシューティング

#### Slack通知が送信されない

1. 環境変数を確認:
```bash
echo $CLAUDE_CODE_SLACK_WEBHOOK_URL
```

2. Webhook URLの形式を確認:
```bash
# 正しい形式: https://hooks.slack.com/services/...
```

3. 手動テスト:
```bash
curl -X POST -H 'Content-Type: application/json' \
  -d '{"text":"Test from Claude Code"}' \
  $CLAUDE_CODE_SLACK_WEBHOOK_URL
```

4. Claude Codeを再起動して設定を反映

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
cp target/release/user-prompt-slack ../bin/
cp target/release/askuser-answer-slack ../bin/
cp target/release/askuser-question-slack ../bin/
cp target/release/exitplanmode-slack ../bin/

# 実行権限を付与
chmod +x ../bin/permission-notification
chmod +x ../bin/task-complete-notification
chmod +x ../bin/user-prompt-slack
chmod +x ../bin/askuser-answer-slack
chmod +x ../bin/askuser-question-slack
chmod +x ../bin/exitplanmode-slack
```

## テスト

### ユニットテスト

テストは`tests/`ディレクトリに配置されています。

```bash
# 全テスト実行
cargo test

# 特定のテストファイルを実行
cargo test --test truncate_content_test
cargo test --test extract_questions_test
```

**テストファイル:**

| ファイル | テスト内容 |
|---------|---------|
| `tests/truncate_content_test.rs` | `truncate_content`関数のテスト（5テスト） |
| `tests/extract_questions_test.rs` | `extract_questions_with_options`関数のテスト（6テスト） |

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

### 手動テスト - user-prompt-slack

```bash
# プロンプト送信のテスト（Slack通知）
echo '{"session_id":"test","cwd":"'$(pwd)'","permission_mode":"bypassPermissions","hook_event_name":"UserPromptSubmit","prompt":"これはテストプロンプトです"}' | \
  ./target/release/user-prompt-slack

# 長いプロンプトのテスト（200文字で切り詰められる）
echo '{"session_id":"test","cwd":"'$(pwd)'","permission_mode":"default","hook_event_name":"UserPromptSubmit","prompt":"'$(printf 'あ%.0s' {1..300})'"}' | \
  ./target/release/user-prompt-slack
```

### 手動テスト - askuser-question-slack

```bash
# 質問通知のテスト
echo '{"session_id":"test","cwd":"'$(pwd)'","tool_name":"AskUserQuestion","tool_input":{"questions":[{"question":"どのフレームワークを使用しますか？","header":"Framework","options":[{"label":"React","description":"人気のUIライブラリ"},{"label":"Vue","description":"プログレッシブフレームワーク"}],"multiSelect":false}]},"tool_response":{}}' | \
  ./target/release/askuser-question-slack
```

### 手動テスト - askuser-answer-slack

```bash
# 回答通知のテスト
echo '{"session_id":"test","cwd":"'$(pwd)'","tool_name":"AskUserQuestion","tool_input":{"questions":[{"question":"どのフレームワークを使用しますか？","header":"Framework","options":[{"label":"React","description":"人気のUIライブラリ"}],"multiSelect":false}]},"tool_response":{"result":[{"question":"どのフレームワークを使用しますか？","answer":["React"]}]}}' | \
  ./target/release/askuser-answer-slack
```

### 手動テスト - exitplanmode-slack

```bash
# プラン完了通知のテスト（~/.claude/plans/に.mdファイルが存在する必要があります）
echo '{"session_id":"test","cwd":"'$(pwd)'","tool_name":"ExitPlanMode","tool_input":{},"tool_response":{}}' | \
  ./target/release/exitplanmode-slack
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
ureq = { version = "2", features = ["json"] }
```

- **serde**: JSON入力のデシリアライズ
- **serde_json**: JSON値の動的処理
- **chrono**: タイムスタンプ生成（ログ用）
- **ureq**: HTTP通信（Slack Webhook用）

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
