#!/usr/bin/env bash
# PR に付いたレビュー（総括コメント・レビュー本体・インラインスレッド）を 1 回の
# GraphQL 呼び出しでまとめて取得し、JSON ファイルに落として索引を標準出力に出す。
#
# 全文を標準出力に流さないのは、レビュー本文が数万文字になることがあり、
# 呼び出し側のコンテキストを一度に食い潰すため。索引を見てから jq / python で
# 必要な部分だけ読むこと。
set -euo pipefail

PR="" ; REPO="" ; OUT="" ; EDITS=30

usage() {
  cat >&2 <<'USAGE'
usage: fetch-pr-review.sh [PR番号] [--repo OWNER/NAME] [--out PATH] [--edits N]

  PR番号      省略時は現在のブランチに紐づく PR
  --repo      省略時は現在のディレクトリのリポジトリ
  --out       JSON の出力先（既定: $TMPDIR/pr-review-<owner>-<repo>-<pr>.json）
  --edits     総括コメントの編集履歴を何世代取るか（新しい順・既定 30）
USAGE
  exit 64
}

while [ $# -gt 0 ]; do
  case "$1" in
    --repo)  REPO="${2:-}"  ; shift 2 ;;
    --out)   OUT="${2:-}"   ; shift 2 ;;
    --edits) EDITS="${2:-}" ; shift 2 ;;
    -h|--help) usage ;;
    -*) echo "不明なオプション: $1" >&2 ; usage ;;
    *)  PR="$1" ; shift ;;
  esac
done

command -v gh      >/dev/null 2>&1 || { echo "gh が無い。https://cli.github.com/ を入れる" >&2 ; exit 69 ; }
command -v python3 >/dev/null 2>&1 || { echo "python3 が無い" >&2 ; exit 69 ; }

if [ -z "$REPO" ]; then
  REPO=$(gh repo view --json nameWithOwner -q .nameWithOwner) || {
    echo "リポジトリを特定できない。--repo OWNER/NAME を渡す" >&2 ; exit 2 ; }
fi
OWNER="${REPO%%/*}" ; NAME="${REPO##*/}"

if [ -z "$PR" ]; then
  # このスキルを使う時点で PR は既にあるはずなので、ここで落ちるのは
  # 「PR が無い」ではなく「今いるブランチから PR を引けない」状態。
  PR=$(gh pr view --json number -q .number 2>/dev/null) || {
    echo "現在のブランチ ($(git branch --show-current 2>/dev/null)) から PR を特定できない。" >&2
    echo "PR 番号を引数で渡すか、PR の head ブランチに切り替える。" >&2
    echo "別リポジトリの PR なら --repo OWNER/NAME も渡す。" >&2
    exit 2 ; }
fi

case "$PR"    in ''|*[!0-9]*) echo "PR 番号が数値でない: $PR" >&2 ; exit 64 ;; esac
case "$EDITS" in ''|*[!0-9]*) echo "--edits が数値でない: $EDITS" >&2 ; exit 64 ;; esac

[ -n "$OUT" ] || OUT="${TMPDIR:-/tmp}/pr-review-${OWNER}-${NAME}-${PR}.json"

gh api graphql \
  -f owner="$OWNER" -f repo="$NAME" -F num="$PR" -F edits="$EDITS" \
  -f query='
query($owner:String!,$repo:String!,$num:Int!,$edits:Int!){
  repository(owner:$owner,name:$repo){
    pullRequest(number:$num){
      number title url state isDraft baseRefName headRefName reviewDecision
      commits(last:1){nodes{commit{oid committedDate}}}
      comments(last:40){nodes{
        databaseId url createdAt updatedAt body
        author{login __typename}
        userContentEdits(first:$edits){totalCount nodes{editedAt diff}}
      }}
      reviews(last:40){nodes{
        author{login __typename} state submittedAt body url
      }}
      reviewThreads(first:100){nodes{
        id isResolved isOutdated path line originalLine
        comments(first:50){nodes{
          databaseId url createdAt body author{login __typename}
        }}
      }}
    }
  }
}' > "$OUT"

OUT="$OUT" python3 - <<'PY'
import json, os, sys

path = os.environ["OUT"]
with open(path) as f:
    doc = json.load(f)

if doc.get("errors"):
    print("GraphQL エラー:", json.dumps(doc["errors"], ensure_ascii=False), file=sys.stderr)
    sys.exit(1)

pr = (doc.get("data") or {}).get("repository", {}).get("pullRequest")
if not pr:
    print("PR を取得できなかった（権限か番号を確認）", file=sys.stderr)
    sys.exit(1)

def author(node):
    a = node.get("author") or {}
    # GraphQL は bot の login から "[bot]" を落とす。REST の claude[bot] と同一人物。
    suffix = "[bot]" if a.get("__typename") == "Bot" else ""
    return (a.get("login") or "ghost") + suffix

head = (pr["commits"]["nodes"] or [{}])[0].get("commit", {})

print(f'PR #{pr["number"]} {pr["title"]}')
print(f'  {pr["url"]}  state={pr["state"]} draft={pr["isDraft"]} '
      f'{pr["headRefName"]} -> {pr["baseRefName"]} decision={pr["reviewDecision"]}')
print(f'  head: {head.get("oid","?")[:8]} committed={head.get("committedDate","?")}')
print(f'  json: {path}')

print("\n[総括コメント] .data.repository.pullRequest.comments.nodes[i]")
for i, c in enumerate(pr["comments"]["nodes"]):
    edits = c["userContentEdits"]["nodes"]
    total = c["userContentEdits"]["totalCount"]
    # 上書き更新されたコメントは createdAt != updatedAt になる。
    # いま見えている本文は最新版で、前の版の指摘は編集履歴にしか残っていない。
    flag = "  ※上書きあり" if total > 1 else ""
    if total > len(edits):
        flag += f"  ※履歴 {total} 件中 {len(edits)} 件のみ取得（--edits {total} で全部取る）"
    print(f'  [{i}] {author(c)} created={c["createdAt"]} updated={c["updatedAt"]} '
          f'本文={len(c["body"] or "")}字 編集履歴={total}件{flag}')
    for j, e in enumerate(edits):
        print(f'        edits[{j}] {e["editedAt"]} {len(e["diff"] or "")}字')

print("\n[レビュー本体] .data.repository.pullRequest.reviews.nodes[i]")
for i, r in enumerate(pr["reviews"]["nodes"]):
    print(f'  [{i}] {author(r)} {r["state"]} {r["submittedAt"]} 本文={len(r["body"] or "")}字')

print("\n[インラインスレッド] .data.repository.pullRequest.reviewThreads.nodes[i]")
for i, t in enumerate(pr["reviewThreads"]["nodes"]):
    cs = t["comments"]["nodes"]
    head_c = cs[0] if cs else {}
    line = t.get("line") or t.get("originalLine")
    state = "resolved" if t["isResolved"] else "open"
    if t["isOutdated"]:
        state += "/outdated"
    print(f'  [{i}] {state} {t["path"]}:{line} thread_id={t["id"]}')
    for c in cs:
        print(f'        {author(c)} {c["createdAt"]} databaseId={c["databaseId"]} '
              f'{len(c["body"] or "")}字')
    if head_c:
        first = (head_c.get("body") or "").strip().splitlines()
        print(f'        冒頭: {first[0][:100] if first else ""}')

if not pr["comments"]["nodes"] and not pr["reviews"]["nodes"] and not pr["reviewThreads"]["nodes"]:
    print("\nレビューは 1 件も付いていない。")
PY
