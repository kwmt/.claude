#!/usr/bin/env node

const { execSync } = require('child_process');

let inputData = '';
process.stdin.on('data', (chunk) => {
  inputData += chunk;
});

process.stdin.on('end', () => {
  try {
    const input = JSON.parse(inputData);

    // モデル名
    const model = input.model?.display_name || 'Unknown';

    // 作業ディレクトリからリポジトリ名を取得
    const cwd = input.workspace?.current_dir || input.cwd || process.cwd();
    let repoName = '';
    let branch = '';
    try {
      process.chdir(cwd);
      const topLevel = execSync('git rev-parse --show-toplevel 2>/dev/null', { encoding: 'utf8' }).trim();
      repoName = require('path').basename(topLevel);
      branch = execSync('git rev-parse --abbrev-ref HEAD 2>/dev/null', { encoding: 'utf8' }).trim();
    } catch (e) {
      repoName = require('path').basename(cwd);
      branch = 'no-git';
    }

    // コンテキスト使用量
    const usedPct = input.context_window?.used_percentage ?? 0;
    const ctxSize = input.context_window?.context_window_size || 200000;
    const ctxSizeK = Math.round(ctxSize / 1000);

    // コンテキスト色
    let ctxColor;
    if (usedPct >= 90) {
      ctxColor = '\x1b[31m'; // 赤
    } else if (usedPct >= 70) {
      ctxColor = '\x1b[33m'; // 黄
    } else {
      ctxColor = '\x1b[32m'; // 緑
    }

    const reset = '\x1b[0m';
    const dim = '\x1b[2m';
    const yellow = '\x1b[33m';
    const cyan = '\x1b[36m';

    // セッションコスト
    const cost = input.cost?.total_cost_usd ?? 0;
    const costStr = `$${cost.toFixed(2)}`;

    // セッション経過時間
    const durationMs = input.cost?.total_duration_ms ?? 0;
    const durationMin = Math.round(durationMs / 60000);
    let durationStr;
    if (durationMin < 60) {
      durationStr = `${durationMin}m`;
    } else {
      const h = Math.floor(durationMin / 60);
      const m = durationMin % 60;
      durationStr = `${h}h${m}m`;
    }

    // 出力: リポジトリ名 | ブランチ名 | コンテキスト使用量 | モデル | コスト(経過時間)
    process.stdout.write(
      `${cyan}${repoName}${reset} ${dim}|${reset} ${yellow}${branch}${reset} ${dim}|${reset} Ctx: ${ctxColor}${usedPct}%${reset}${dim}/${ctxSizeK}k${reset} ${dim}|${reset} ${model} ${dim}|${reset} ${costStr} ${dim}(${durationStr})${reset}`
    );
  } catch (e) {
    process.stdout.write(`Claude Code | Error: ${e.message}`);
  }
});
