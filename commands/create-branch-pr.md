# Create Branch and Pull Request

ブランチ作成からPR作成までを一括で行うコマンド

## 実行手順

1. **変更の確認**
   - `git status` で未コミットの変更を確認
   - 変更がある場合はコミットする

2. **ブランチ作成**
   - 現在のブランチがmain/stagingの場合、新しいブランチを作成
   - ブランチ名は変更内容に基づいて自動生成（例: `feat/add-login-feature`）
   - ブランチ名の形式: `{type}/{description}`
     - type: feat, fix, refactor, docs, chore, test など

3. **リモートへプッシュ**
   - `git push -u origin {branch-name}` でリモートにプッシュ

4. **PR作成**
   - プロジェクトの `.claude/commands/pr.md` が存在する場合は `/pr` コマンドを使用
   - 存在しない場合は `gh pr create --draft` でドラフトPRを作成
   - ベースブランチは `staging` または `main` を使用
   - PRテンプレートは `.github/pull_request_template.md` を参照

## PRタイトル形式

絵文字 + スコープ + 説明の形式を使用:
- `✨(feature): 新機能の説明`
- `🐛(fix): バグ修正の説明`
- `♻️(refactor): リファクタリングの説明`
- `📝(docs): ドキュメント更新`
- `🔧(config): 設定変更`
- `✅(test): テスト追加・修正`

## 使用例

```bash
# 基本的な使い方
/create-branch-pr

# 実行されるコマンドの流れ
git checkout -b feat/new-feature
git push -u origin feat/new-feature

# プロジェクトに /pr コマンドがある場合
/pr

# /pr コマンドがない場合
gh pr create --draft --title "✨(scope): Title" --base main
```
