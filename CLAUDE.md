日本語で説明してください。

## Git Workflow Rules

plan modeから実行モードに切り替える際、現在のブランチが main, develop, staging のいずれかである場合:
1. 作業開始前に新しいブランチを作成する（例: feat/xxx, fix/xxx）
2. 作業完了後にPRを作成する（/create-branch-pr または /pr コマンドを使用）