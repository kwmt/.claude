変更をコミットして push する。

`commit` スキル（`~/.claude/skills/commit/SKILL.md`）を読み、その哲学・書式・実行フローに従ってコミットし、
最後に現在のブランチを push する。

- 対象は **modified / staged の変更のみ**。untracked ファイルは、コミットすべきか確認してから扱う。
- 保護ブランチ（main / develop / staging 等）に居る場合は push せず、先にブランチを切る必要があることを報告する。
