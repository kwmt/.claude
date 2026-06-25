---
name: lookingback
description: >
  appbrewメンバーの定期振り返りレポートを自動生成するスキル。GitHubの活動（PR・Issue・コメント）とSlackの活動を収集し、
  振り返りサマリーとしてSlack CanvasとNotionページに出力する。
  「振り返り」「lookingback」「ふりかえり」「活動まとめ」「週報」「月報」「レポート作成」
  「GitHub活動」「Slackまとめ」といったキーワードで発動すること。
  ユーザーが期間を指定して自分やチームメンバーの活動を振り返りたいときに積極的に使う。
---

# Lookingback — 振り返りレポート生成スキル

appbrewの開発メンバーの活動（GitHub + Slack）を収集して、振り返りレポートを自動生成するスキル。

## 概要

このスキルは以下の流れで動作する:

1. **期間の確定** — ユーザーが指定した期間（デフォルト: 過去2週間）を確定
2. **GitHub活動の収集** — appbrew org内での対象ユーザーの活動を収集
3. **Slack活動の収集** — 対象ユーザーに関連するSlackメッセージを収集
4. **レポート生成** — 収集した情報を構造化してまとめる
5. **出力** — Slack CanvasとNotionページに出力

## デフォルト設定

対象ユーザーのデフォルト値は以下の通り（ユーザーが別途指定した場合はそちらを優先）:

| 項目 | デフォルト値 |
|------|-------------|
| GitHubユーザー名 | `kwmt` |
| GitHub org | `appbrew` |
| Slackユーザー名 | `kawamoto` |
| デフォルト期間 | 過去2週間 |
| Slack出力チャンネル | `#workspace_kawamoto` |
| Notion親ページID | `4b341e98ef6b49649e032391b0f9f528` |

## Step 1: 期間の確定

ユーザーのプロンプトから期間を読み取る。明示的な指定がなければデフォルト（過去2週間）を使う。

期間の指定例:
- 「先週の振り返り」→ 先週月曜〜日曜
- 「3月の振り返り」→ 3月1日〜3月末日
- 「2026-03-01から2026-03-14」→ そのまま使用
- 指定なし → 今日から14日前〜今日

期間が確定したら、ISO 8601形式の `START_DATE` と `END_DATE` を決定する。

## Step 2: GitHub活動の収集

GitHub REST APIを使って、appbrew org内の対象ユーザーの活動を収集する。

### GitHub API呼び出し方法

`scripts/fetch_github.py` を実行して活動データを取得する:

```bash
python3 /path/to/skill/scripts/fetch_github.py \
  --username kwmt \
  --org appbrew \
  --start-date 2026-03-13 \
  --end-date 2026-03-27
```

このスクリプトは GitHub REST API を使い、以下のデータを JSON で標準出力に返す:
- 作成したPR一覧（タイトル、URL、リポジトリ名、状態、作成日）
- 作成したIssue一覧（タイトル、URL、リポジトリ名、状態、作成日）
- PRレビューコメント（コメント内容、対象PR、リポジトリ名）
- Issueコメント（コメント内容、対象Issue、リポジトリ名）

**重要**: GitHub APIにはトークンが必要。環境変数 `GITHUB_TOKEN` が設定されていない場合、スクリプトはエラーメッセージを出す。

トークンが未設定の場合の対応:
1. ユーザーに GitHub Personal Access Token (classic) の作成を案内:
   - https://github.com/settings/tokens → "Generate new token (classic)"
   - スコープ: `repo` (private repoにアクセスする場合) または `public_repo` (public repoのみ)
2. トークンの設定方法を案内:
   - Claude Code: `export GITHUB_TOKEN=ghp_xxxxx` を `.bashrc` や `.zshrc` に追加
   - Cowork: セッション内で `export GITHUB_TOKEN=ghp_xxxxx` を実行

### フォールバック: Chrome MCP

GitHub APIが使えない場合（トークンなし、rate limit等）、Chrome MCP を使ってGitHub検索ページからデータを取得する:

1. `mcp__Claude_in_Chrome__navigate` で以下のURLにアクセス:
   - PR: `https://github.com/search?q=org:appbrew+author:kwmt+created:START_DATE..END_DATE&type=pullrequests`
   - Issue: `https://github.com/search?q=org:appbrew+author:kwmt+created:START_DATE..END_DATE&type=issues`
   - コメント: `https://github.com/search?q=org:appbrew+commenter:kwmt+created:START_DATE..END_DATE&type=issues`
2. `mcp__Claude_in_Chrome__get_page_text` でページ内容を取得
3. 検索結果をパースしてデータを構造化

この方法ではコメントの本文は取得しにくいため、PRやIssueのタイトルとリポジトリ名の一覧が主な出力になる。コメント内容まで見たい場合は個別のPR/Issueページにアクセスする。

### 収集するデータ

以下の4カテゴリを収集する。**必ずリポジトリ名を含めること**:

1. **作成したPR** — `author:kwmt` で検索
2. **作成したIssue** — `author:kwmt` で検索
3. **PRレビューコメント** — `commenter:kwmt` で検索
4. **Issueコメント** — `commenter:kwmt` で検索

## Step 3: Slack活動の収集

Slack MCP を使って対象ユーザーの活動を収集する。

### 収集手順

**重要**: Slack検索では `from:@username` 形式は動作しない場合がある。必ず `from:<@USER_ID>` 形式を使うこと。

1. まず `slack_search_users` で対象ユーザーのSlack IDを取得:
   ```
   query: "kawamoto"
   ```
   → デフォルトのSlack ID: `U02LJFD3AR2`

2. `slack_search_public_and_private` で対象ユーザーが送信したメッセージを検索:
   ```
   query: "from:<@U02LJFD3AR2> after:START_DATE before:END_DATE"
   sort: "timestamp"
   include_context: false
   response_format: "concise"
   ```
   ※ 結果が多い場合はcursorを使ってページネーション（最大3ページ程度）

3. `slack_search_public_and_private` で対象ユーザーがメンションされたメッセージを検索:
   ```
   query: "<@U02LJFD3AR2> after:START_DATE before:END_DATE"
   sort: "timestamp"
   channel_types: "public_channel,private_channel"
   include_context: false
   response_format: "concise"
   ```
   ※ `to:` ではなくユーザーIDをキーワードとして検索する方が、チャンネル内のメンションを拾いやすい
   ※ DMやbot メッセージは除外するために `channel_types` を指定

4. 重要そうなスレッドは `slack_read_thread` で詳細を取得

### 収集時のポイント

- チャンネル名を記録する（どのチャンネルでの会話かわかるように）
- bot メッセージは除外 (`include_bots: false`)
- コンテキストは簡潔に (`response_format: "concise"`)
- メッセージ数が多い場合は、主要な議論やトピックに絞ってまとめる

## Step 4: レポート生成

収集した情報を以下の構造でまとめる。読みやすさと振り返りやすさを重視する。

### レポート構造

レポートは3部構成にする。最も重要なのは冒頭の「3分まとめ」で、朝会やスタンドアップで口頭共有できる簡潔な振り返りを提供する。

```markdown
# 振り返りレポート: {ユーザー名}
## 期間: {START_DATE} 〜 {END_DATE}

---

## 3分で話せるまとめ

この期間の活動を3分で口頭共有できるようにまとめたもの。箇条書きではなく、自然な話し言葉に近い形で書く。
以下の観点を含める:

1. **やったこと**: 具体的な成果物や対応を2-3個ピックアップ。「◯◯の改修をして、△△の問題を解決しました」のように具体的に。
2. **進行中・次にやること**: 今進めていること、来週以降の予定。
3. **相談・共有事項**: 他チームとの議論、意思決定が必要なもの、ブロッカーがあれば。

書き方のポイント:
- **箇条書きを積極的に使う**。読みやすさ最優先。
- 各項目は1-2文で簡潔にまとめる。
- 「PRを25件出しました」のような数字の羅列は避ける。数字は補足程度に。
- 代わりに「何をなぜやったか」「それによってどうなったか」を中心に書く。
- Slackで議論になったトピックのうち、チームに共有すべきものがあれば含める。

---

## GitHub活動詳細

### 作成したPR ({件数}件)
| リポジトリ | タイトル | 状態 | 作成日 |
|-----------|---------|------|--------|
| repo-name | PR title | merged/open/closed | 2026-03-15 |

### 作成したIssue ({件数}件)
| リポジトリ | タイトル | 状態 | 作成日 |
|-----------|---------|------|--------|
| repo-name | Issue title | open/closed | 2026-03-15 |

### PRレビュー・コメント ({件数}件)
各PRへのレビューコメントを要約。
- **repo-name#123** (PRタイトル): コメント内容の要約

### Issueコメント ({件数}件)
各Issueへのコメントを要約。
- **repo-name#456** (Issueタイトル): コメント内容の要約

---

## Slack活動詳細

Slackの活動はチャンネルごとではなく、**トピック（案件・課題）単位**でグループ化する。
1つのトピックが複数チャンネルにまたがることもあるので、チャンネル名はラベルとして付ける程度にする。

### まとめ方のルール

- トピック単位でグループ化する（例: 「mov動画対応」「開封率Redash連携」）
- 各トピックに以下を含める:
  - 概要（何についての議論か）
  - 経緯（誰が何を言って、どう進んだか）
  - 結論・現状（決まったこと、未決のこと）
  - 関連チャンネル名
- 出退勤（#attendance_general）やbotメッセージは除外
- 自分が送ったメッセージとメンションされたメッセージの両方を統合して1つの流れとして記述

### テンプレート

#### 🔹 {トピック名}
**チャンネル**: #channel-name
**関係者**: 名前1, 名前2
**概要**: 1-2文でトピックの説明
**経緯**: 議論の流れを時系列で簡潔に（3-5文）
**結論・現状**: 決まったこと、未決のこと、次のアクション
```

レポート内容は日本語で書く。GitHub URLやSlackリンクは省略せずに含める。

## Step 5: 出力

### 5a. Slack Canvas出力

`#workspace_kawamoto` チャンネルにCanvasを作成または更新する。

1. まず `slack_search_channels` で `workspace_kawamoto` チャンネルのIDを取得
2. 既存のCanvasがあるか確認:
   - `slack_search_public` で `creator:@me type:canvas in:#workspace_kawamoto` を検索
   - 同じ期間のCanvasが見つかったら `slack_update_canvas` で更新（`action: "replace"`）
   - 見つからなければ `slack_create_canvas` で新規作成
3. Canvasのタイトルは `振り返り {START_DATE}〜{END_DATE}` とする
4. Canvas作成/更新後、チャンネルにCanvasへのリンクをメッセージとして投稿

### 5b. Notion出力

親ページ「振り返りメモ」(ID: `4b341e98ef6b49649e032391b0f9f528`) の下に子ページを作成する。

1. `notion-create-pages` で子ページを作成:
   - parent: `{ "page_id": "4b341e98ef6b49649e032391b0f9f528" }`
   - title: `振り返り {START_DATE}〜{END_DATE}`
   - content: Step 4で生成したレポート内容（Notion Markdown形式）
   - icon: `📝`

2. 同じ期間の既存ページがないか `notion-fetch` で親ページを確認し、重複があればユーザーに確認する

**Notion Markdown形式の注意点**: Notion独自のMarkdown仕様に従うこと。テーブル、ヘッダー（最大###まで）、リンクなどを使用。`notion://docs/enhanced-markdown-spec` でフルスペックを確認できる。

## スケジュール実行との連携

このスキルは `/schedule` スキルと組み合わせて定期実行できるように設計されている。

### スケジュール設定例

ユーザーが「毎日午前9時に振り返りを実行」のようなリクエストをした場合:

1. `schedule` スキルを使って定期タスクを作成する
2. タスクのプロンプトには以下のように設定:
   ```
   lookingbackスキルを使って、過去2週間の振り返りレポートを生成して。
   GitHubユーザー: kwmt、Slackユーザー: kawamoto。
   Slack Canvas (#workspace_kawamoto) と Notion (振り返りメモページの子ページ) に出力して。
   ```
3. スケジュール実行時は自律的に動作する必要があるため、ユーザーへの確認なしで全ステップを完了すること
4. エラーが発生した場合はSlackの `#workspace_kawamoto` チャンネルにエラー内容をメッセージとして投稿する

### 自律実行モード

スケジュールから起動された場合（またはユーザーが「自動で全部やって」と指示した場合）:
- 期間の確認をスキップし、デフォルト値を使用
- 重複チェックで既存ページが見つかっても確認なしで新規作成
- 全ステップを一気に完了する

## エラーハンドリング

- **GitHub API エラー**: トークンがない場合 → ユーザーに `GITHUB_TOKEN` 環境変数の設定を依頼。Rate limit の場合 → Chrome MCP フォールバック
- **Slack検索結果が0件**: 期間や検索クエリを広げてリトライ、それでもなければその旨をレポートに記載
- **Notion API エラー**: エラー内容をユーザーに伝え、代わりにMarkdownファイルとして出力を提案
