#!/usr/bin/env node

const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');

// 標準入力からJSONを読み込む
let inputData = '';
process.stdin.on('data', (chunk) => {
  inputData += chunk;
});

process.stdin.on('end', () => {
  try {
    const input = JSON.parse(inputData);

    // モデル名を取得
    const model = input.model?.display_name || 'Unknown Model';

    // セッションIDを取得（先頭8文字）
    const sessionId = (input.session_id || 'unknown').substring(0, 8);

    // 現在のディレクトリを取得
    const cwd = input.workspace?.current_dir || input.cwd || process.cwd();

    // Gitブランチを取得
    let branch = 'no-git';
    try {
      process.chdir(cwd);
      branch = execSync('git rev-parse --abbrev-ref HEAD 2>/dev/null', { encoding: 'utf8' }).trim();
    } catch (e) {
      // Gitリポジトリではない場合
    }

    // トークン情報を取得
    let totalTokens = 0;
    const transcriptPath = input.transcript_path;

    if (transcriptPath && fs.existsSync(transcriptPath)) {
      try {
        const content = fs.readFileSync(transcriptPath, 'utf8');
        const lines = content.trim().split('\n');

        // 各行のJSONからトークン情報を集計
        for (const line of lines) {
          if (!line.trim()) continue;

          try {
            const entry = JSON.parse(line);
            const usage = entry.message?.usage;

            if (usage) {
              totalTokens += usage.input_tokens || 0;
              totalTokens += usage.output_tokens || 0;
              totalTokens += usage.cache_creation_input_tokens || 0;
              totalTokens += usage.cache_read_input_tokens || 0;
            }
          } catch (e) {
            // 行のパースエラーは無視
          }
        }
      } catch (e) {
        // ファイル読み込みエラーは無視
      }
    }

    // コンテキスト上限と圧縮閾値
    const CONTEXT_LIMIT = 200000;
    const COMPACTION_THRESHOLD = CONTEXT_LIMIT * 0.8; // 160,000

    // 使用率を計算（圧縮閾値基準）
    const percentage = Math.min(100, Math.round((totalTokens / COMPACTION_THRESHOLD) * 100));

    // 色を決定
    let color;
    if (percentage >= 90) {
      color = '\x1b[31m'; // 赤
    } else if (percentage >= 70) {
      color = '\x1b[33m'; // 黄
    } else {
      color = '\x1b[32m'; // 緑
    }
    const reset = '\x1b[0m';
    const yellow = '\x1b[33m';

    // トークン数を k 単位で表示
    const tokensInK = Math.round(totalTokens / 1000);
    const compactionInK = Math.round(COMPACTION_THRESHOLD / 1000);
    const limitInK = Math.round(CONTEXT_LIMIT / 1000);

    // ステータスラインを出力
    process.stdout.write(
      `💰 Model: ${model} | Session: ${sessionId}... | Branch: ${yellow}${branch}${reset} | Context: ${color}${percentage}%${reset} (${tokensInK}k / ${compactionInK}k of ${limitInK}k)`
    );
  } catch (e) {
    // エラー時はデフォルト表示
    process.stdout.write(`💰 Claude Code | Error: ${e.message}`);
  }
});
