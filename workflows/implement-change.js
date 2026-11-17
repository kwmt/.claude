// implement-change — 機能実装 / リファクタリングを多段で進める、プロジェクト非依存のワークフロー。
//
// 流れ: Scout (リポジトリ調査) → Design → [refactor のみ Safety net] → Implement → Test
//       → Self-review loop (レビュー → 判定 → 修正 → 検証 → コミット を収束まで)
//
// プロジェクト固有の規約・コマンドはここにハードコードしない。Scout が実行時のリポジトリから
// 読み取り、後段の全エージェントに渡す。
//
// セルフレビューの中身 (観点・分類基準・コミット規則) は次のスキルが正で、ここには書き写さない。
// 各エージェントが実行時に Read する。節番号を変えるときはこのファイルも合わせること。
//   ~/.claude/skills/self-review/SKILL.md       §1 観点表と但し書き / §2 集約
//   ~/.claude/skills/self-review-loop/SKILL.md  §1-2 分類 / §1-3〜1-5 修正・検証・コミット / §2 打ち切り
//
// args:
//   文字列                      … やりたいこと (機能説明 / リファクタリングの狙い)
//   { task, mode, scale, context, until, maxRounds, models }
//     mode      'feature' | 'refactor' | 'auto' (既定 auto: Scout が判定)
//     scale     'small' | 'standard' | 'large' | 'auto' (既定 auto: Scout が判定)
//     context   仕様・デザイン値・issue 本文など、エージェントに渡したい追加情報
//     until     'blocker' | 'major' (既定 major)   セルフレビューで 0 にする severity
//     maxRounds セルフレビューの最大ラウンド数 (既定 4)
//     models    段ごとの上書き。例: { implement: { model: 'fable', effort: 'high' } }
export const meta = {
  name: 'implement-change',
  description:
    '機能実装・リファクタリングを 調査→設計→実装→テスト→セルフレビューループ の多段で進める (プロジェクト非依存)',
  whenToUse:
    '複数ファイルにまたがる機能追加やリファクタリングを、設計からセルフレビューでの指摘解消まで一気に進めたいとき。args にやりたいことを文字列で渡す (mode / scale は自動判定。{ task, mode: "refactor" } のように指定もできる)。1〜2 ファイルの小さな変更には使わない。',
  phases: [
    { title: 'Scout', detail: '規約・検証コマンド・ブランチ状態を調べ、mode と scale を判定', model: 'sonnet' },
    { title: 'Design', detail: '観点別の設計案を並列生成 → 批判的に統合して確定 (small は 1 体で設計)' },
    { title: 'Safety net', detail: 'refactor のみ: 現状の振る舞いを固定するテストを先に揃える', model: 'sonnet' },
    { title: 'Implement', detail: '確定設計に沿って実装しコミット (モデルは scale で決まる)' },
    { title: 'Test', detail: 'feature: テスト追加 / refactor: 振る舞いが変わっていないことの確認', model: 'sonnet' },
    { title: 'Self-review loop', detail: '多観点レビュー → 判定 → 修正 → 検証 → コミット を収束まで' },
  ],
}

// ---- 入力 ------------------------------------------------------------------
const input = typeof args === 'string' ? { task: args } : args && typeof args === 'object' ? args : {}
const task = typeof input.task === 'string' ? input.task.trim() : ''
if (!task) {
  log('args にやりたいことが渡されていません。例: Workflow({ name: "implement-change", args: "プロフィール画面を追加する" })')
  return { error: 'no task description provided' }
}
const pick = (v, allowed, fallback) => (allowed.includes(v) ? v : fallback)
const requestedMode = pick(input.mode, ['feature', 'refactor'], 'auto')
const requestedScale = pick(input.scale, ['small', 'standard', 'large'], 'auto')
const until = pick(input.until, ['blocker', 'major'], 'major')
const maxRounds = Number.isInteger(input.maxRounds) && input.maxRounds > 0 ? input.maxRounds : 4
const extraContext = typeof input.context === 'string' && input.context.trim() ? input.context.trim() : ''
const overrides = input.models && typeof input.models === 'object' ? input.models : {}

// ---- モデル選定 --------------------------------------------------------------
// 根拠: https://claude.dev/blog/which-claude-model-should-you-build-with/
//   - 調査・レビュー・下書きなど繰り返しのエージェント作業 → Sonnet 5.5 / medium
//   - 仕様が固まった範囲の明確なコーディング (テスト作成など) → Sonnet 5.5 / medium
//   - 日常的なエンジニアリング・複数ファイルの機能実装 → Opus 5.5 / medium
//   - 大規模リファクタ・移行・アーキテクチャ設計 → Fable 5.2 / medium
//   - 「コード編集と判断は主力モデルに残し、小さいモデルは誤りにすぐ気づける作業に使う」
//     → 設計の確定・指摘の真偽判定は Opus 以上、実装と修正は scale に応じた主力モデル
// effort は記事のとおり全段 medium から始める。品質が足りない段は、記事の順序で上げる:
//   (1) その段が自分の成果を検証できているか (テスト・ビルド) → (2) effort を 1 段上げる
//   → (3) モデルを 1 つ上げ、effort は下げて始める。args.models で段ごとに上書きできる。
// セッションのモデルを継承させない (model を必ず明示する) のは、セッションが Fable のとき
// 調査やレビューまで Fable で走って費用が数倍になるのを避けるため。
const MAIN_BY_SCALE = {
  small: { model: 'sonnet', effort: 'medium' },
  standard: { model: 'opus', effort: 'medium' },
  large: { model: 'fable', effort: 'medium' },
}
const stageModel = (stage, scale) => {
  const base = {
    scout: { model: 'sonnet', effort: 'medium' },
    designDraft: { model: 'sonnet', effort: 'medium' },
    designDecide: { model: scale === 'large' ? 'fable' : 'opus', effort: 'medium' },
    safetyNet: { model: 'sonnet', effort: 'medium' },
    implement: MAIN_BY_SCALE[scale],
    test: { model: 'sonnet', effort: 'medium' },
    review: { model: 'sonnet', effort: 'medium' },
    triage: { model: 'opus', effort: 'medium' },
    fix: MAIN_BY_SCALE[scale],
  }[stage]
  return { ...base, ...(overrides[stage] && typeof overrides[stage] === 'object' ? overrides[stage] : {}) }
}

// ---- 全エージェント共通の運用ガード -------------------------------------------
const OPS = [
  '## 実行ガード',
  '- このワークフローはバックグラウンドで動く。ユーザーに質問できない。判断がつかないことは勝手に決めず、返り値の該当欄 (openQuestions / unresolved など) に書いて返す。',
  '- 1 つで数分かかりうるコマンド (テストの全体実行、フルビルド、依存の再取得、開発サーバーの起動) を避ける。検証は「リポジトリ情報」にある対象を絞ったコマンドを使う。絞れない場合だけ全体実行し、それも現実的でなければ静的検証 (lint / 型チェック) に留めて、その旨を返す。',
  '- push・PR 作成・リモートへの書き込みはしない。',
  '- 返り値に、ローカルの絶対パス・秘匿値やトークンの実値・実在する個人情報を書かない (パスはリポジトリ相対)。',
].join('\n')

// ============================================================================
// Phase: Scout — リポジトリから規約・コマンド・状態を読み取る
// ============================================================================
phase('Scout')

const SCOUT_SCHEMA = {
  type: 'object',
  additionalProperties: false,
  required: [
    'mode', 'scale', 'scaleReason', 'stack', 'baseBranch', 'diffBaseRef', 'currentBranch',
    'onProtectedBranch', 'suggestedBranch', 'blockers', 'conventionSources', 'conventionNotes',
    'commands', 'slowCommands', 'relatedFiles',
  ],
  properties: {
    mode: { type: 'string', enum: ['feature', 'refactor'], description: '振る舞いを増やす/変えるなら feature、振る舞いを保ったまま構造を変えるなら refactor' },
    scale: { type: 'string', enum: ['small', 'standard', 'large'] },
    scaleReason: { type: 'string' },
    stack: { type: 'string', description: '言語・フレームワーク・構成の要約 (2〜3 文)' },
    baseBranch: { type: 'string', description: 'origin/ を含まない生のブランチ名' },
    diffBaseRef: { type: 'string', description: 'git merge-base に渡せる ref。リモートがあれば origin/<base>、無ければ <base>' },
    currentBranch: { type: 'string' },
    onProtectedBranch: { type: 'boolean', description: '現在のブランチが baseBranch、または main / develop / staging' },
    suggestedBranch: { type: 'string', description: 'feat/xxx や refactor/xxx 形式の作業ブランチ名' },
    blockers: { type: 'array', items: { type: 'string' }, description: 'このまま進めるべきでない理由。無ければ空配列' },
    conventionSources: { type: 'array', items: { type: 'string' }, description: '規約が書かれたファイルのリポジトリ相対パス' },
    conventionNotes: { type: 'string', description: 'このタスクに関係する規約と、隣接コードから読み取れる暗黙の慣習' },
    commands: {
      type: 'object',
      additionalProperties: false,
      required: ['test', 'testScoped', 'lint', 'typecheck', 'format', 'codegen'],
      properties: {
        test: { type: 'string' },
        testScoped: { type: 'string', description: '単一ファイル / 単一パッケージに絞ったテストの実行方法' },
        lint: { type: 'string' },
        typecheck: { type: 'string' },
        format: { type: 'string' },
        codegen: { type: 'string' },
      },
    },
    slowCommands: { type: 'array', items: { type: 'string' }, description: '数分かかる・ハングしやすいので避けるべきコマンド' },
    relatedFiles: {
      type: 'array',
      items: {
        type: 'object',
        additionalProperties: false,
        required: ['path', 'why'],
        properties: { path: { type: 'string' }, why: { type: 'string' } },
      },
    },
  },
}

const scout = await agent(
  `## やりたいこと\n${task}\n\n` +
    (extraContext ? `## 追加情報\n${extraContext}\n\n` : '') +
    `あなたは後続の設計・実装・レビュー担当のために、このリポジトリの前提を調べる担当。読み取り専用で、ファイルもブランチも変更しない。\n\n` +
    `調べること:\n` +
    `1. 規約: プロジェクトの CLAUDE.md / AGENTS.md、.claude/rules/、CONTRIBUTING.md、アーキテクチャ文書、lint・formatter 設定。見つかったファイルのパスと、このタスクに関係する要点を返す。存在しない規約を作らない。\n` +
    `2. 暗黙の慣習: 変更対象になりそうなコードの隣接ファイルを読み、命名・配置・層構造・テストの書き方を要約する。\n` +
    `3. 検証コマンド: テスト・lint・型チェック・フォーマット・コード生成のコマンドを README、CI 設定、package.json / Makefile / pubspec などから特定する。対象を絞った実行方法 (testScoped) を必ず探す。該当が無い項目は空文字にする。数分かかる・ハングしやすいコマンドは slowCommands に挙げる。\n` +
    `4. git の状態: 現在のブランチ、ベースブランチ (PR があればその base、無ければ origin/HEAD、それも無ければ main / develop / staging のうち存在するもの)、作業ブランチ名の案。\n` +
    `5. blockers: 追跡対象ファイルに未コミットの変更がある (このワークフローのコミットに無関係な変更が混ざる)、detached HEAD、やりたいことが曖昧すぎて設計できない、などがあれば具体的に書く。未追跡ファイルがあるだけなら blocker にしない。\n` +
    `6. 関連ファイル: このタスクで読む・変更することになりそうなファイル。\n\n` +
    `判定:\n` +
    `- mode: ${requestedMode === 'auto' ? '外から観測できる振る舞いを増やす・変えるなら feature、振る舞いを保ったまま構造を変えるなら refactor。両方を含むなら feature。' : `呼び出し側の指定どおり ${requestedMode} を返す。`}\n` +
    `- scale: ${requestedScale === 'auto' ? 'small = 1〜2 ファイル・数十行で仕様が明確 / standard = 複数ファイルにまたがる通常の機能追加・整理 / large = モジュール横断の再設計、移行、公開インターフェースの変更を伴うもの。迷ったら standard。' : `呼び出し側の指定どおり ${requestedScale} を返す。`}\n\n` +
    OPS,
  { label: 'scout', phase: 'Scout', schema: SCOUT_SCHEMA, ...stageModel('scout', 'standard') },
)

if (!scout) return { error: 'リポジトリの調査に失敗しました', task }
if (scout.blockers.length) {
  log(`中断: ${scout.blockers.join(' / ')}`)
  return { error: '開始できない状態です', task, blockers: scout.blockers }
}

const mode = requestedMode === 'auto' ? scout.mode : requestedMode
const scale = requestedScale === 'auto' ? scout.scale : requestedScale
const isRefactor = mode === 'refactor'
log(`mode=${mode} scale=${scale} (${scout.scaleReason}) / 実装モデル=${stageModel('implement', scale).model}`)

const cmd = scout.commands
const REPO = [
  '## リポジトリ情報 (Scout の調査結果。疑わしければ自分で読んで確かめてよい)',
  `- 構成: ${scout.stack}`,
  `- 規約が書かれたファイル: ${scout.conventionSources.length ? scout.conventionSources.join(', ') : '(見つからなかった)'}`,
  `- 規約・慣習の要点:\n${scout.conventionNotes}`,
  `- 検証コマンド: テスト(絞り込み)=\`${cmd.testScoped || '不明'}\` / テスト(全体)=\`${cmd.test || '不明'}\` / lint=\`${cmd.lint || '不明'}\` / 型チェック=\`${cmd.typecheck || '不明'}\` / format=\`${cmd.format || '不明'}\` / コード生成=\`${cmd.codegen || 'なし'}\``,
  `- 避けるコマンド: ${scout.slowCommands.length ? scout.slowCommands.join(' / ') : '(特になし)'}`,
  `- ベースブランチ: ${scout.baseBranch} / 作業ブランチ名の案: ${scout.suggestedBranch}`,
  `- 関連ファイル:\n${scout.relatedFiles.map((f) => `  - ${f.path} — ${f.why}`).join('\n') || '  (なし)'}`,
].join('\n')

const TASK =
  `## やりたいこと (${isRefactor ? 'リファクタリング' : '機能実装'})\n${task}` +
  (extraContext ? `\n\n## 追加情報\n${extraContext}` : '')

const COMMIT = [
  '## コミットの規則',
  `- 最初のコミットの前に現在のブランチを確認する。${scout.baseBranch} / main / develop / staging のいずれかにいるなら、作業ブランチ \`${scout.suggestedBranch}\` を作って切り替えてからコミットする (既に作業ブランチにいればそのまま使う)。`,
  '- コミットの書式は ~/.claude/skills/commit/SKILL.md を Read して従う (gitmoji、本文には「なぜ」)。',
  '- 自分が変更・作成したファイルだけをパス指定で stage する。`git add -A` / `git add .` は使わない (無関係な未追跡ファイルを巻き込む)。',
  '- 検証 (テスト・lint・型チェック) が通っていない状態ではコミットしない。`--amend` と push はしない。',
].join('\n')

// ============================================================================
// Phase: Design — 観点別に並列で設計し、批判的に統合して確定
// ============================================================================
phase('Design')

const FILE_ITEMS = {
  type: 'array',
  items: {
    type: 'object',
    additionalProperties: false,
    required: ['path', 'action', 'purpose'],
    properties: {
      path: { type: 'string' },
      action: { type: 'string', enum: ['create', 'modify', 'move', 'delete'] },
      purpose: { type: 'string' },
    },
  },
}

const DRAFT_SCHEMA = {
  type: 'object',
  additionalProperties: false,
  required: ['summary', 'files', 'steps', 'risks'],
  properties: {
    summary: { type: 'string' },
    files: FILE_ITEMS,
    steps: { type: 'array', items: { type: 'string' } },
    risks: { type: 'array', items: { type: 'string' } },
  },
}

const DESIGN_SCHEMA = {
  type: 'object',
  additionalProperties: false,
  required: ['summary', 'plan', 'files', 'invariants', 'testPlan', 'conventionsToFollow', 'openRisks', 'openQuestions'],
  properties: {
    summary: { type: 'string' },
    plan: { type: 'array', items: { type: 'string' }, description: '実装手順 (順序付き)。refactor では各手順の後にテストが通る粒度にする' },
    files: FILE_ITEMS,
    invariants: { type: 'array', items: { type: 'string' }, description: '変えてはいけない振る舞い・公開インターフェース・互換性。refactor では必須' },
    testPlan: { type: 'array', items: { type: 'string' } },
    conventionsToFollow: { type: 'array', items: { type: 'string' } },
    openRisks: { type: 'array', items: { type: 'string' } },
    openQuestions: { type: 'array', items: { type: 'string' }, description: 'ユーザーに確認が要る事項。仮置きした前提も書く' },
  },
}

const LENSES = isRefactor
  ? [
      { key: 'behavior', hint: '振る舞いの保存を最優先。外から観測できる振る舞い (公開 API・出力・副作用・エラー) を洗い出し、何を変えてはいけないか、既存テストがそれをどこまで固定しているかを明らかにする。' },
      { key: 'steps', hint: '安全な進め方を最優先。各ステップの後でテストが通る小さな手順に分解し、順序と、途中で止めても壊れない区切りを設計する。' },
      { key: 'target', hint: '到達点の構造を最優先。責務の分け方・依存方向・重複の解消を具体的に描く。ただし今回の狙いに必要な範囲に留め、ついでの作り替えを含めない。' },
    ]
  : [
      { key: 'mvp', hint: '最小構成で動くことを最優先。過剰な抽象化・先回りの設計を避ける。' },
      { key: 'architecture', hint: '既存の層構造・依存方向・責務分離との整合を最優先。この変更が既存設計のどこに収まるべきかを詰める。' },
      { key: 'reuse', hint: '既存の関数・コンポーネント・ユーティリティの再利用と重複排除を最優先。新規追加を最小にする。' },
    ]

// small は並列の設計案を作らない (差分に対して過剰。1 体が調べて確定する)。
const drafts =
  scale === 'small'
    ? []
    : (
        await parallel(
          LENSES.map((l) => () =>
            agent(
              `${TASK}\n\n${REPO}\n\n` +
                `あなたは「${l.key}」の観点で設計案を出す担当。${l.hint}\n` +
                `関連する既存コードと規約ファイルを自分で読んで現状を把握したうえで、変更するファイルと責務、進める順序、想定リスクを具体的に返せ。読み取り専用で、コードは変更しない。\n\n${OPS}`,
              { label: `design:${l.key}`, phase: 'Design', schema: DRAFT_SCHEMA, ...stageModel('designDraft', scale) },
            ).then((d) => (d ? { lens: l.key, ...d } : null)),
          ),
        )
      ).filter(Boolean)

if (scale !== 'small' && drafts.length < LENSES.length) {
  log(`設計案 ${LENSES.length} 件中 ${drafts.length} 件のみ取得。得られた案で確定設計を作る`)
}

const design = await agent(
  `${TASK}\n\n${REPO}\n\n` +
    (drafts.length
      ? `以下は観点別の設計案。あなたは悪魔の代弁者を兼ねたテックリード。各案を批判的に評価し (規約違反・過剰設計・抜け・スコープ逸脱を指摘)、実際のコードを読んで裏を取ったうえで、最良の案を軸に他案の良い点を統合した「確定設計」を 1 つにまとめよ。\n\n` +
        drafts.map((d) => `### 案 (${d.lens})\n${JSON.stringify(d, null, 2)}`).join('\n\n')
      : `関連する既存コードと規約ファイルを読み、この変更の「確定設計」を 1 つ作れ。小さな変更なので、設計も必要最小限でよい。`) +
    `\n\n` +
    (isRefactor
      ? `これはリファクタリングである。invariants に「変えてはいけない振る舞い」を具体的に列挙し、plan は各手順の後でテストが通る粒度にする。振る舞いの変更を伴う作業が紛れていたら、plan から外して openQuestions に書く。\n`
      : `testPlan には、追加するロジックごとに正常系・異常系・境界値のどれを確かめるかを書く。\n`) +
    `読み取り専用で、コードは変更しない。\n\n${OPS}`,
  { label: 'design:decide', phase: 'Design', schema: DESIGN_SCHEMA, ...stageModel('designDecide', scale) },
)

if (!design) return { error: '設計を確定できませんでした', task, mode, scale }
log(`確定設計: ${design.summary}`)

const DESIGN = `## 確定設計\n${JSON.stringify(design, null, 2)}`

const WORK_SCHEMA = {
  type: 'object',
  additionalProperties: false,
  required: ['summary', 'files', 'validation', 'commits', 'unresolved'],
  properties: {
    summary: { type: 'string' },
    files: { type: 'array', items: { type: 'string' } },
    validation: {
      type: 'object',
      additionalProperties: false,
      required: ['commands', 'passed', 'detail'],
      properties: {
        commands: { type: 'array', items: { type: 'string' } },
        passed: { type: 'boolean' },
        detail: { type: 'string', description: '結果の要約。実行できなかった検証があればその理由' },
      },
    },
    commits: { type: 'array', items: { type: 'string' }, description: '作成したコミットの短縮 SHA と件名' },
    unresolved: { type: 'array', items: { type: 'string' }, description: '設計どおりにできなかった点・判断を保留した点' },
  },
}

// ============================================================================
// Phase: Safety net — refactor のみ。変更前に、現状の振る舞いをテストで固定する
// ============================================================================
let safetyNet = null
if (isRefactor) {
  phase('Safety net')
  safetyNet = await agent(
    `${TASK}\n\n${REPO}\n\n${DESIGN}\n\n` +
      `リファクタリングに着手する前の安全網を整える担当。本体のコードはまだ変更しない。\n` +
      `1. 確定設計の invariants それぞれについて、既存テストが固定しているかを確かめる。\n` +
      `2. 固定されていないものには、**現在の実装の振る舞いをそのまま**記録するテスト (characterization test) を追加する。現状の振る舞いがバグに見えても、ここでは直さず unresolved に書く。\n` +
      `3. 関連するテストを実行し、変更前の時点で通ることを確認する。既に落ちているテストがあれば、名前を unresolved に書く (リファクタリング起因の失敗と区別するため)。\n` +
      `4. テストを追加した場合だけコミットする。既存テストで足りているなら、何も変更せずその旨を返す。\n\n` +
      `${COMMIT}\n\n${OPS}`,
    { label: 'safety-net', phase: 'Safety net', schema: WORK_SCHEMA, ...stageModel('safetyNet', scale) },
  )
  if (!safetyNet) log('安全網の整備に失敗。既存テストのみを頼りに進める')
}

// ============================================================================
// Phase: Implement — 確定設計に沿って実装 (単一エージェントで一貫性を保つ)
// ============================================================================
phase('Implement')

const impl = await agent(
  `${TASK}\n\n${REPO}\n\n${DESIGN}\n\n` +
    (safetyNet ? `## 安全網の整備結果\n${JSON.stringify(safetyNet, null, 2)}\n\n` : '') +
    `この確定設計に沿って実装せよ。\n` +
    `- 既存の構成・命名・層構造に従う。規約ファイルと隣接コードを読んでから書く。\n` +
    `- 設計に無い変更を足さない。設計が実コードと合わない箇所は、最小限の調整に留めて unresolved に書く。\n` +
    (isRefactor
      ? `- 外から観測できる振る舞いを変えない (確定設計の invariants)。plan の手順ごとに関連テストを実行し、通ることを確かめながら進める。テストの期待値を書き換えて通すのは、振る舞いではなく内部構造に依存したテストの場合だけにし、その都度 unresolved に理由を書く。\n` +
        `- 既存のバグや改善点に気づいても直さない。unresolved に書く。\n`
      : `- テストはこの段階では書かない (次の担当が、実装ではなく仕様からテストを書く)。既存テストを壊した場合は直す。\n`) +
    `- コード生成が必要なら実行する。生成物を手で書かない。\n` +
    `- 実装後、format・lint・型チェックと、関連する既存テストを実行して通す。\n` +
    `- 通ったらコミットする。意図が分かれるなら複数コミットに分ける。\n\n` +
    `${COMMIT}\n\n${OPS}`,
  { label: 'implement', phase: 'Implement', schema: WORK_SCHEMA, ...stageModel('implement', scale) },
)

if (!impl) return { error: '実装に失敗しました', task, mode, scale, design }

// ============================================================================
// Phase: Test — feature: テスト追加 / refactor: 振る舞いが変わっていないことの確認
// ============================================================================
phase('Test')

const tests = await agent(
  `${TASK}\n\n${REPO}\n\n${DESIGN}\n\n## 実装の結果\n${JSON.stringify(impl, null, 2)}\n\n` +
    (isRefactor
      ? `リファクタリング後の確認担当。\n` +
        `1. 変更したファイルに関係するテストを実行し、通ることを確かめる。落ちた場合は、振る舞いが変わったのか、テストが内部構造に依存していたのかを切り分ける。前者は本体を直し、後者はテストを直す。\n` +
        `2. 新しく切り出した単位 (関数・クラス・モジュール) のうち、ロジックを持ち、既存テストで間接的にも通っていないものにだけテストを追加する。\n` +
        `3. 移動・改名で不要になったテストや重複したテストがあれば整理する。\n`
      : `テスト作成担当。実装の内部ではなく、確定設計の testPlan と「やりたいこと」の仕様からテストを書く。\n` +
        `1. 新しく追加・変更されたロジックに、正常系・異常系・境界値のテストを書く。既存のテストの置き場所・命名・モックの流儀に合わせる。\n` +
        `2. 対象を絞って実行し、通ることを確かめる。実装のバグを見つけたら、テストを実装に合わせるのではなく実装を直す。\n` +
        `3. ロジックを持たない変更でテストが不要なら、その旨を返して終了してよい。\n`) +
    `変更があればコミットする。\n\n${COMMIT}\n\n${OPS}`,
  { label: 'test', phase: 'Test', schema: WORK_SCHEMA, ...stageModel('test', scale) },
)

if (!tests) log('テスト段が失敗。テスト未整備のままセルフレビューに進む')

// ============================================================================
// Phase: Self-review loop — self-review-loop スキルをワークフローの制御構造に載せたもの
//   ラウンド 1: 多観点レビュー (並列) → 判定
//   ラウンド 2 以降: 前ラウンドの修正だけを 1 体が再確認して判定 (全観点を再起動しない)
//   各ラウンド: fix に分類された対象 severity の指摘を修正 → 検証 → コミット
//   打ち切り条件はスキル §2 と同じ。判定はスクリプト側で決定的に行う。
// ============================================================================
phase('Self-review loop')

const SELF_REVIEW = '~/.claude/skills/self-review/SKILL.md'
const SELF_REVIEW_LOOP = '~/.claude/skills/self-review-loop/SKILL.md'
const DIFF_CMD = `git diff $(git merge-base ${scout.diffBaseRef} HEAD)`
const targetSeverities = until === 'blocker' ? ['blocker'] : ['blocker', 'major']
const inTarget = (i) => targetSeverities.includes(i.severity)

const FINDING_PROPS = {
  file: { type: 'string', description: 'リポジトリ相対パス' },
  line: { type: 'integer' },
  perspective: { type: 'string', enum: ['security', 'quality', 'convention', 'simplify', 'requirements', 'checklist'] },
  severity: { type: 'string', enum: ['blocker', 'major', 'minor', 'nit'] },
  confidence: { type: 'string', enum: ['high', 'medium', 'low'] },
  summary: { type: 'string' },
  detail: { type: 'string' },
  fix: { type: 'string' },
}
const FINDING_REQUIRED = ['file', 'line', 'perspective', 'severity', 'confidence', 'summary', 'detail', 'fix']

const FINDINGS_SCHEMA = {
  type: 'object',
  additionalProperties: false,
  required: ['findings', 'coverage'],
  properties: {
    findings: {
      type: 'array',
      items: { type: 'object', additionalProperties: false, required: FINDING_REQUIRED, properties: FINDING_PROPS },
    },
    coverage: { type: 'string', description: '何を見たか、見られなかったものは何か' },
  },
}

const TRIAGE_ITEMS = {
  type: 'array',
  items: {
    type: 'object',
    additionalProperties: false,
    required: [...FINDING_REQUIRED, 'id', 'origin', 'classification', 'reason'],
    properties: {
      ...FINDING_PROPS,
      id: { type: 'string' },
      origin: {
        type: 'string',
        enum: ['review', 'unresolved', 'regression', 'new'],
        description: 'review=ラウンド 1 のレビュー由来 / unresolved=前ラウンドで直したはずが直っていない / regression=修正が生んだ新しい問題 / new=それ以外で新たに気づいた問題',
      },
      classification: { type: 'string', enum: ['fix', 'dismiss-false-positive', 'dismiss-intentional', 'defer', 'needs-user'] },
      reason: { type: 'string', description: '分類の根拠。dismiss は何を確認してそう判断したか' },
    },
  },
}
const TRIAGE_SCHEMA = {
  type: 'object',
  additionalProperties: false,
  required: ['items', 'addedLines'],
  properties: {
    items: TRIAGE_ITEMS,
    addedLines: { type: 'integer', description: `\`${DIFF_CMD} --shortstat\` の追加行数` },
  },
}

const FIX_SCHEMA = {
  type: 'object',
  additionalProperties: false,
  required: ['fixed', 'notFixed', 'validation', 'commit', 'addedLines'],
  properties: {
    fixed: {
      type: 'array',
      items: {
        type: 'object',
        additionalProperties: false,
        required: ['id', 'how'],
        properties: { id: { type: 'string' }, how: { type: 'string' } },
      },
    },
    notFixed: {
      type: 'array',
      items: {
        type: 'object',
        additionalProperties: false,
        required: ['id', 'reason'],
        properties: { id: { type: 'string' }, reason: { type: 'string' } },
      },
    },
    validation: WORK_SCHEMA.properties.validation,
    commit: { type: 'string', description: '作成したコミットの短縮 SHA と件名。コミットしなかったら空文字' },
    addedLines: { type: 'integer', description: `修正後の \`${DIFF_CMD} --shortstat\` の追加行数` },
  },
}

// 観点の束ね方は self-review §0-3 の規模表に合わせる。
const BUNDLES = {
  small: [['security', 'quality', 'convention', 'simplify', 'requirements', 'checklist']],
  standard: [
    ['security', 'quality'],
    ['convention', 'simplify'],
    ['requirements', 'checklist'],
  ],
  large: [['security'], ['quality'], ['convention'], ['simplify'], ['requirements'], ['checklist']],
}[scale]

const REFACTOR_REVIEW_NOTE = isRefactor
  ? `\nこれはリファクタリングである。requirements 観点では「外から観測できる振る舞いが変わっていないか」「確定設計の invariants を破っていないか」「リファクタリングに機能変更が紛れていないか」を見る。checklist 観点では、移動・改名に追随していない参照 (import、設定、ドキュメント、テスト) を見る。\n`
  : ''

const REVIEW_CONTEXT =
  `${TASK}\n\n${REPO}\n\n${DESIGN}\n\n` +
  `## レビュー対象\nこのブランチの変更全体。差分は \`${DIFF_CMD}\` で取る (コミット済み・未コミットの両方を含む二点 diff)。`

const reviewRound = await parallel(
  BUNDLES.map((lenses) => () =>
    agent(
      `${REVIEW_CONTEXT}\n\n` +
        `あなたはセルフレビューの「${lenses.join(' + ')}」観点の担当。\n` +
        `まず ${SELF_REVIEW} を Read し、§1 の観点表から担当観点の focus を、同じ §1 の「共通の但し書き」から守るべきことを読み取って従う (気づいた問題は全件返す・file:line に紐づける・周辺コードを読んでから指摘する・読み取り専用)。\n` +
        `規約違反を指摘するときは、根拠にした規約ファイルか隣接コードを detail に示す。根拠を示せない好みは nit にする。` +
        REFACTOR_REVIEW_NOTE +
        `\n${OPS}`,
      { label: `review:${lenses.join('+')}`, phase: 'Self-review loop', schema: FINDINGS_SCHEMA, ...stageModel('review', scale) },
    ).then((r) => (r ? { lenses, ...r } : null)),
  ),
)

const reviewed = reviewRound.filter(Boolean)
const failedLenses = BUNDLES.filter((_, i) => !reviewRound[i]).flat()
if (failedLenses.length) log(`レビューに失敗した観点: ${failedLenses.join(', ')} (この観点は未確認として報告する)`)

const ledger = { fixed: [], dismissed: [], deferred: [], needsUser: [], remaining: [], minor: [] }
const commits = []
const validations = []
let stop = null
let round = 1
let baselineAddedLines = null
let regressionStreak = 0
let decided = []

const record = (items) => {
  for (const i of items) {
    if (i.classification === 'defer') ledger.deferred.push(i)
    else if (i.classification.startsWith('dismiss')) ledger.dismissed.push(i)
    else if (i.classification === 'needs-user') ledger.needsUser.push(i)
    else if (!inTarget(i)) ledger.minor.push(i)
  }
  decided = [...ledger.dismissed, ...ledger.deferred, ...ledger.needsUser].map((i) => ({
    file: i.file,
    line: i.line,
    summary: i.summary,
    classification: i.classification,
    reason: i.reason,
  }))
}

const TRIAGE_RULES =
  `${SELF_REVIEW_LOOP} を Read し、§1-2 の分類基準に従って各指摘を fix / dismiss-false-positive / dismiss-intentional / defer のいずれかに分類する。` +
  `分類の前に必ず該当ファイルを自分で Read して真偽を確かめる (レビュー担当が古い前提で読んでいる可能性も疑う)。` +
  `スキルが「ユーザーに聞く」としている、判断がつかない blocker は needs-user にする (このワークフローからは質問できないため)。勝手に dismiss しない。`

let items = []
if (!reviewed.length) {
  stop = 'review-failed'
} else {
  const rawFindings = reviewed.flatMap((r) => r.findings)
  log(`ラウンド 1: 指摘 ${rawFindings.length} 件を判定`)
  const triage = await agent(
    `${REVIEW_CONTEXT}\n\n` +
      `あなたはセルフレビューの指摘を集約・判定する担当。読み取り専用で、コードは変更しない。\n` +
      `1. ${SELF_REVIEW} を Read し、§2 の手順で重複排除と信頼度フィルタを行う (confidence=low は落とす。ただし blocker は残す)。\n` +
      `2. ${TRIAGE_RULES}\n` +
      `3. 残した各指摘に id を \`R1-1\`, \`R1-2\`, ... と振り、origin は review にする。落とした指摘は返さない。\n` +
      `4. \`${DIFF_CMD} --shortstat\` を実行し、追加行数を addedLines に入れる。\n` +
      REFACTOR_REVIEW_NOTE +
      `\n## レビュー担当からの指摘\n${JSON.stringify(rawFindings, null, 2)}\n\n${OPS}`,
    { label: 'triage:round-1', phase: 'Self-review loop', schema: TRIAGE_SCHEMA, ...stageModel('triage', scale) },
  )
  if (!triage) {
    stop = 'triage-failed'
  } else {
    items = triage.items
    baselineAddedLines = triage.addedLines
  }
}

while (!stop) {
  record(items)
  const toFix = items.filter((i) => i.classification === 'fix' && inTarget(i))
  if (!toFix.length) {
    stop = ledger.needsUser.some(inTarget) ? 'needs-user' : 'converged'
    break
  }
  // 最終ラウンドでは修正しない。直しても再確認できず、未確認の修正を残すことになるため。
  if (round >= maxRounds) {
    ledger.remaining.push(...toFix.map((i) => ({ ...i, why: `--max-rounds=${maxRounds} に到達` })))
    stop = 'max-rounds'
    break
  }

  log(`ラウンド ${round}: ${toFix.length} 件を修正`)
  const fix = await agent(
    `${TASK}\n\n${REPO}\n\n${DESIGN}\n\n` +
      `あなたはセルフレビューで fix と判定された指摘を直す担当。\n` +
      `${SELF_REVIEW_LOOP} を Read し、§1-3 (修正)・§1-4 (テストを実際に走らせる)・§1-5 (コミット) に従う。要点:\n` +
      `- 渡された指摘だけを直す。指摘の行だけを機械的に直さず、原因を理解して直す。同じ原因が他にあればそこも直す。\n` +
      `- 指摘箇所のコードを消して黙らせる・無関係な書き換えをする、をしない。読んで誤検知だと判断したら直さず notFixed に理由を書く。\n` +
      (isRefactor ? `- リファクタリングの修正なので、外から観測できる振る舞いを変えない。\n` : '') +
      `- 修正後に検証コマンドを実際に実行する。落ちたら通るまで直す。通らないままならコミットせず validation.passed=false で返す。\n` +
      `- 通ったら、このラウンドの修正を 1 コミットにまとめる。本文に、どの指摘 (file:line / severity / 要旨) をどう直したかを書く。新規ファイルは stage を忘れない。\n` +
      `- コミット後に \`${DIFF_CMD} --shortstat\` の追加行数を addedLines に入れる。\n\n` +
      `## 直す指摘 (ラウンド ${round})\n${JSON.stringify(toFix, null, 2)}\n\n${COMMIT}\n\n${OPS}`,
    { label: `fix:round-${round}`, phase: 'Self-review loop', schema: FIX_SCHEMA, ...stageModel('fix', scale) },
  )

  if (!fix) {
    ledger.remaining.push(...toFix.map((i) => ({ ...i, why: '修正エージェントが失敗' })))
    stop = 'fix-failed'
    break
  }
  validations.push({ round, ...fix.validation })
  const fixedIds = new Set(fix.fixed.map((f) => f.id))
  const fixedNow = toFix.filter((i) => fixedIds.has(i.id)).map((i) => ({ ...i, how: fix.fixed.find((f) => f.id === i.id).how, round }))
  const skipped = toFix.filter((i) => !fixedIds.has(i.id))
  ledger.remaining.push(
    ...skipped.map((i) => ({ ...i, why: (fix.notFixed.find((n) => n.id === i.id) || {}).reason || '修正されなかった (理由の報告なし)' })),
  )
  if (!fix.validation.passed) {
    ledger.remaining.push(...fixedNow.map((i) => ({ ...i, why: '修正後の検証が通らず、コミットされていない' })))
    stop = 'validation-failed'
    break
  }
  ledger.fixed.push(...fixedNow)
  if (fix.commit) commits.push({ round, commit: fix.commit })
  if (!fixedNow.length) {
    stop = 'no-progress'
    break
  }
  if (baselineAddedLines && fix.addedLines > baselineAddedLines * 2) {
    log(`差分が膨らんだ: 追加 ${baselineAddedLines} 行 → ${fix.addedLines} 行`)
    stop = 'diff-bloat'
    break
  }

  round++
  const recheck = await agent(
    `${REVIEW_CONTEXT}\n\n` +
      `あなたはセルフレビューのラウンド ${round} の担当。前ラウンドの修正だけを確認する (差分全体の再レビューはしない)。読み取り専用で、コードは変更しない。\n` +
      `1. 下の「前ラウンドで直した指摘」それぞれについて該当箇所を Read し、実際に直っているかを確かめる。直っていなければ origin=unresolved で返す。\n` +
      `2. 修正コミット (${fix.commit || '直近のコミット'}) の差分を読み、修正が新しい問題を生んでいないかを確かめる。特にセキュリティと正しさ。見つけたら origin=regression で返す。修正と無関係に気づいた問題は origin=new。\n` +
      `3. 「判断済みの事項」は再提出しない。ただし理由が誤っていると考える根拠が新たにあるなら、その根拠を detail に添えて再提出してよい。\n` +
      `4. ${TRIAGE_RULES}\n` +
      `5. id は \`R${round}-1\`, \`R${round}-2\`, ... と振る。問題が無ければ items は空配列でよい。無理に絞り出さない。\n` +
      `6. \`${DIFF_CMD} --shortstat\` の追加行数を addedLines に入れる。\n` +
      REFACTOR_REVIEW_NOTE +
      `\n## 前ラウンドで直した指摘\n${JSON.stringify(fixedNow.map((i) => ({ id: i.id, file: i.file, line: i.line, severity: i.severity, summary: i.summary, how: i.how })), null, 2)}\n\n` +
      `## 判断済みの事項\n${JSON.stringify(decided, null, 2)}\n\n${OPS}`,
    { label: `recheck:round-${round}`, phase: 'Self-review loop', schema: TRIAGE_SCHEMA, ...stageModel('triage', scale) },
  )

  if (!recheck) {
    stop = 'recheck-failed'
    break
  }
  items = recheck.items
  const actionable = items.filter((i) => i.classification === 'fix' && inTarget(i))
  if (actionable.some((i) => i.origin === 'unresolved')) {
    record(items)
    ledger.remaining.push(...actionable.map((i) => ({ ...i, why: '前ラウンドの修正で直っていない' })))
    stop = 'resubmitted'
    break
  }
  regressionStreak = actionable.some((i) => i.origin === 'regression') ? regressionStreak + 1 : 0
  if (regressionStreak >= 2) {
    record(items)
    ledger.remaining.push(...actionable.map((i) => ({ ...i, why: '修正が新しい指摘を生むのが 2 回続いた' })))
    stop = 'oscillation'
    break
  }
}

// 一部の観点が見られていないなら「収束」と言わない (見ていないものを clean と報告しない)。
if (stop === 'converged' && failedLenses.length) stop = 'converged-partial'

const STOP_TEXT = {
  converged: `収束: ${targetSeverities.join(' / ')} の未処理指摘が 0 になった`,
  'converged-partial': `部分的に収束: 見られた観点では ${targetSeverities.join(' / ')} の未処理指摘が 0 になったが、${failedLenses.join(', ')} 観点はレビューできていない`,
  'needs-user': '中断: ユーザーの判断が要る指摘が残っている',
  'max-rounds': `中断: 最大ラウンド数 (${maxRounds}) に到達した`,
  resubmitted: '中断: 直したはずの指摘が直っていなかった。修正が効いていない',
  oscillation: '中断: 修正が新しい指摘を生むのが 2 回続いた。局所修正ではなく設計の見直しが要る可能性がある',
  'diff-bloat': '中断: レビュー起因の修正で差分が開始時の 2 倍を超えた。スコープが膨らんでいる',
  'validation-failed': '中断: 修正後の検証が通らなかった。未コミットの変更が作業ツリーに残っている',
  'no-progress': '中断: 修正担当が 1 件も直せなかった',
  'review-failed': '中断: レビュー担当が全員失敗し、セルフレビューを実施できていない',
  'triage-failed': '中断: 指摘の判定に失敗し、セルフレビューの結果が得られていない',
  'fix-failed': '中断: 修正担当が失敗した。作業ツリーに中途半端な変更が残っている可能性がある',
  'recheck-failed': '中断: 修正の再確認に失敗した。最後のラウンドの修正は未確認',
}
log(STOP_TEXT[stop])

const brief = (i) => ({
  where: `${i.file}:${i.line}`,
  severity: i.severity,
  perspective: i.perspective,
  summary: i.summary,
})

return {
  task,
  mode,
  scale,
  models: {
    designDecide: stageModel('designDecide', scale).model,
    implement: stageModel('implement', scale).model,
    review: stageModel('review', scale).model,
    triage: stageModel('triage', scale).model,
  },
  design: { summary: design.summary, invariants: design.invariants, openRisks: design.openRisks, openQuestions: design.openQuestions },
  safetyNet,
  implemented: impl,
  tests,
  selfReview: {
    result: STOP_TEXT[stop],
    converged: stop === 'converged',
    rounds: round,
    until,
    // 収束の判定スコープ。ラウンド 2 以降は修正箇所だけを見ている。
    scope:
      round > 1
        ? 'ラウンド 1 で差分全体を見たうえで、以降は修正箇所だけを再確認した結果'
        : 'ラウンド 1 で差分全体を見た結果',
    uncoveredLenses: failedLenses,
    coverage: reviewed.map((r) => ({ lenses: r.lenses.join(' + '), coverage: r.coverage })),
    fixed: ledger.fixed.map((i) => ({ ...brief(i), how: i.how, round: i.round })),
    dismissed: ledger.dismissed.map((i) => ({ ...brief(i), classification: i.classification, reason: i.reason })),
    deferred: ledger.deferred.map((i) => ({ ...brief(i), reason: i.reason })),
    needsUser: ledger.needsUser.map((i) => ({ ...brief(i), detail: i.detail, reason: i.reason })),
    remaining: ledger.remaining.map((i) => ({ ...brief(i), why: i.why })),
    minorAndNit: ledger.minor.map((i) => ({ ...brief(i), fix: i.fix })),
    validations,
    commits,
  },
  followUps: [
    'push はしていない。内容を確認してから push・PR 作成を行う (/create-branch-pr)。',
    '検証は対象を絞って実行している。テスト全体・フルビルドは CI かローカルで別途確認する。',
    'design.openQuestions と selfReview.needsUser は、ユーザーの判断が要る事項。dismissed は却下理由を確認し、納得できなければ覆す。',
  ],
}
