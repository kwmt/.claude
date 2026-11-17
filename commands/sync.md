---
description: ベースブランチ（develop / staging / main）を remote と同期し、作業ブランチを rebase する。マージ済みの作業ブランチは削除する。switch を付けると、ベースブランチに移って pull と Rails のセットアップを行う。
argument-hint: "[main|staging|develop] [switch]"
---

ベースブランチを remote と同期する。引数: `$ARGUMENTS`

## ベースブランチの決め方

- 引数にブランチ名（`main` / `staging` / `stg` / `develop` など）があればそれを使う。`stg` は `staging` と読み替える。
- 無ければ、origin に存在するものを `develop` → `staging` → `main` の順に探し、最初に見つかったものを使う。

## 通常の動作

「同期」は local を remote に合わせる向きで行う。push はしない。

1. `git fetch --prune origin` で remote の状態を取得する。
2. 追跡ファイルに未コミットの変更（staged / unstaged）があれば、ここで止まって、どうするかを確認する。捨てたり stash したり commit したりしない。未追跡ファイルだけなら進めてよい。止まるときは、未コミットの変更（ファイルと概要）、今いるブランチが `origin/<ベース>` から何コミット進んで何コミット遅れているか、続行したら rebase と削除のどちらになるか（手順 3 の判定は読み取りだけなので先に行ってよい）を伝える。
3. 今いるブランチがベースブランチ以外なら、マージ済みかどうかを判定する（下の「マージ済みの判定」）。
4. マージ済みなら、ベースブランチに checkout し、local のベースブランチを `origin/<ベース>` に fast-forward してから、local の作業ブランチを削除する。squash マージ済みなら `git branch -D` を使ってよい。remote のブランチは削除しない。
5. マージ済みでなければ、今いるブランチを離れずに local のベースブランチを `origin/<ベース>` に fast-forward し（`git fetch origin <ベース>:<ベース>`。local に無ければこれで作られる）、今いるブランチをそのベースブランチの上に rebase する（`git rebase <ベース>`）。コンフリクトしたら勝手に解決せず、状況を伝えて止まる。
6. 今いるブランチがベースブランチなら、`origin/<ベース>` に fast-forward するだけでよい。

local のベースブランチが remote より進んでいる、または分岐していて fast-forward できないときは、何も変更せずに状況を伝えて止まる。

### マージ済みの判定

次のどれかが成り立てばマージ済みとする。

- 作業ブランチの先端が `origin/<ベース>` に含まれている（通常のマージ）
- `gh` で、そのブランチの PR が MERGED になっている
- 作業ブランチを `origin/<ベース>` にマージしても差分が出ない（内容がすべて取り込み済み。squash マージ）

`gh` が使えない、PR が見つからないときは、残りの条件だけで判定する。どれも成り立たなければ未マージとして扱う。

## `switch` が付いているとき

rebase もブランチの削除もせず、ベースブランチに移って環境を整える。元いたブランチには触れない。

1. 追跡ファイルに未コミットの変更があれば、通常の動作と同じく止まって確認する。
2. ベースブランチに checkout する。
3. `git pull --ff-only` する。fast-forward できないときは、通常の動作と同じく状況を伝えて止まる。
4. Rails プロジェクト（`Gemfile` と `bin/rails` がある）なら `bundle install` を実行し、成功したら `bin/rails db:migrate` を実行する。失敗したコマンドがあれば、後続を実行せずにエラーを伝えて止まる。回避策を探して環境を書き換えない。

## 終わったら伝えること

通常の動作でも `switch` でも、最後に次を伝える。途中で止まったときも、そこまでに行ったことを同じ形で伝える。

- 使ったベースブランチと、そう決めた理由
- 行ったこと（fast-forward の範囲、rebase したか、削除したか）と、今いるブランチ
- ブランチを削除したときは、マージ済みと判断した根拠と、削除したブランチの先端のコミットハッシュ
- `switch` のときは、`bundle install` と `bin/rails db:migrate` それぞれの結果（実行していないならその理由）
- push していないこと（rebase 後の force push は頼まれたときだけ行う）

## 例

- `/sync` — develop か staging（無ければ main）と同期
- `/sync main` — main と同期
- `/sync staging` — staging と同期
- `/sync staging switch` — staging に移って pull、bundle install、db:migrate
