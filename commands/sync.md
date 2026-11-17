---
description: ベースブランチ（develop / staging / main）を remote と同期し、作業ブランチを rebase する。マージ済みの作業ブランチは削除する。
argument-hint: "[main|staging|develop] [switch]"
---

ベースブランチを remote と同期する。引数: `$ARGUMENTS`

## ベースブランチの決め方

- 引数にブランチ名（`main` / `staging` / `stg` / `develop` など）があればそれを使う。`stg` は `staging` と読み替える。
- 無ければ、origin に存在するものを `develop` → `staging` → `main` の順に探し、最初に見つかったものを使う。

## 通常の動作

1. remote のベースブランチを local と同期する。
2. 今いるブランチがベースブランチ以外で、すでにマージ済み（squash マージを含む。PR の状態でも確かめる）なら、ベースブランチに checkout してからそのブランチを削除する。
3. マージ済みでなければ、同期したベースブランチを今いるブランチに rebase する。コンフリクトしたら勝手に解決せず、状況を伝えて止まる。

未コミットの変更があるときは、捨てたり stash したりせずに、先にどうするかを確認する。

## `switch` が付いているとき

rebase はせず、ベースブランチに移って環境を整える。

1. ベースブランチに checkout する。
2. `git pull` する。
3. Rails プロジェクト（`Gemfile` と `bin/rails` がある）なら `bundle install` と `bin/rails db:migrate` を実行する。

## 例

- `/sync` — develop か staging（無ければ main）と同期
- `/sync main` — main と同期
- `/sync staging` — staging と同期
- `/sync staging switch` — staging に移って pull、bundle install、db:migrate
