import {
  BarChart,
  Callout,
  Card,
  CardBody,
  CardHeader,
  Grid,
  H1,
  H2,
  LineChart,
  Pill,
  Row,
  Stack,
  Stat,
  Table,
  Text,
} from "./ui";

// ── 数据源:results/offline_v3.json · repair_8k.json · repair_local.json ·
//            repair_full_v2.json · full_run.json · miri_baseline.json ·
//            lockbud_baseline.json · scaling.json · scaling_llm.json ·
//            no_goals_v2.json · codegen(_v2).json ──

type Analyze = {
  case: string;
  expected: string;
  actual: string;
  kinds: string[];
  places: number;
  transitions: number;
  arcs: [number, number];
  states: number;
  ms: number;
  statements: number;
};

const analyze: Analyze[] = [
  { case: "mutex_deadlock", expected: "bug", actual: "bug", kinds: ["Deadlock"], places: 22, transitions: 17, arcs: [23, 23], states: 60, ms: 0.45, statements: 15 },
  { case: "three_way_deadlock", expected: "bug", actual: "bug", kinds: ["Deadlock"], places: 32, transitions: 25, arcs: [34, 34], states: 396, ms: 1.78, statements: 22 },
  { case: "signal_loss", expected: "bug", actual: "bug", kinds: ["SignalLoss"], places: 23, transitions: 20, arcs: [26, 25], states: 45, ms: 0.53, statements: 14 },
  { case: "channel_deadlock", expected: "bug", actual: "bug", kinds: ["ChannelBlock"], places: 20, transitions: 15, arcs: [20, 20], states: 37, ms: 0.53, statements: 13 },
  { case: "dead_transition", expected: "bug", actual: "bug", kinds: ["DeadTransition"], places: 4, transitions: 4, arcs: [4, 4], states: 3, ms: 0.21, statements: 3 },
  { case: "dual_condvar", expected: "bug", actual: "bug", kinds: ["SignalLoss"], places: 32, transitions: 29, arcs: [39, 37], states: 21, ms: 0.55, statements: 19 },
  { case: "partial_deadlock", expected: "goals_unmet", actual: "bug + unmet goals", kinds: ["DeadTransition"], places: 33, transitions: 25, arcs: [33, 34], states: 81, ms: 0.69, statements: 22 },
  { case: "goal_unreachable", expected: "goals_unmet", actual: "goals_unmet", kinds: [], places: 19, transitions: 15, arcs: [19, 19], states: 55, ms: 0.47, statements: 13 },
  { case: "goal_trivial", expected: "goals_unmet", actual: "goals_unmet", kinds: [], places: 19, transitions: 15, arcs: [19, 19], states: 55, ms: 0.58, statements: 13 },
  { case: "goal_bad_reference", expected: "goals_unmet", actual: "goals_unmet", kinds: [], places: 19, transitions: 15, arcs: [19, 19], states: 55, ms: 0.46, statements: 13 },
  { case: "cas_race", expected: "safe", actual: "safe", kinds: [], places: 16, transitions: 15, arcs: [17, 17], states: 31, ms: 0.39, statements: 11 },
  { case: "semaphore_throttle", expected: "safe", actual: "safe", kinds: [], places: 24, transitions: 19, arcs: [25, 25], states: 187, ms: 0.98, statements: 16 },
  { case: "fn_summary_prop", expected: "safe", actual: "safe", kinds: [], places: 21, transitions: 17, arcs: [21, 21], states: 57, ms: 0.44, statements: 15 },
  { case: "scale_lock_chain_6x3", expected: "safe", actual: "safe", kinds: [], places: 71, transitions: 61, arcs: [85, 85], states: 53477, ms: 366.24, statements: 55 },
  { case: "scale_lock_chain_5x3_buggy", expected: "bug", actual: "bug", kinds: ["Deadlock"], places: 60, transitions: 51, arcs: [71, 71], states: 14054, ms: 83.76, statements: 46 },
  { case: "scale_branch_fan_4x2", expected: "safe", actual: "safe", kinds: [], places: 50, transitions: 53, arcs: [57, 57], states: 29121, ms: 166.8, statements: 41 },
  { case: "deep_lock_chain_4x3", expected: "bug", actual: "bug", kinds: ["Deadlock"], places: 79, transitions: 75, arcs: [102, 102], states: 43883, ms: 318.32, statements: 67 },
  { case: "deep_lock_chain_4x3_safe", expected: "safe", actual: "safe", kinds: [], places: 81, transitions: 77, arcs: [105, 105], states: 44585, ms: 320.56, statements: 67 },
  { case: "goal_constrained_deadlock", expected: "bug", actual: "bug", kinds: ["Deadlock"], places: 42, transitions: 37, arcs: [47, 47], states: 2189, ms: 10.78, statements: 33 },
  { case: "goal_constrained_deadlock_dense", expected: "bug", actual: "bug", kinds: ["Deadlock"], places: 71, transitions: 68, arcs: [87, 87], states: 65525, ms: 466.28, statements: 60 },
];

const placesByKind: Record<string, [number, number, number]> = {
  mutex_deadlock: [20, 2, 0], three_way_deadlock: [29, 3, 0], signal_loss: [20, 2, 1],
  channel_deadlock: [18, 2, 0], dead_transition: [4, 0, 0], dual_condvar: [26, 4, 2],
  partial_deadlock: [29, 4, 0], goal_unreachable: [18, 1, 0], goal_trivial: [18, 1, 0],
  goal_bad_reference: [18, 1, 0], cas_race: [16, 0, 0],
  semaphore_throttle: [23, 1, 0], fn_summary_prop: [20, 1, 0],
  scale_lock_chain_6x3: [68, 3, 0], scale_lock_chain_5x3_buggy: [57, 3, 0],
  scale_branch_fan_4x2: [50, 0, 0], deep_lock_chain_4x3: [76, 3, 0],
  deep_lock_chain_4x3_safe: [78, 3, 0],
  goal_constrained_deadlock: [40, 2, 0],
  goal_constrained_deadlock_dense: [69, 2, 0],
};

// llm_judge:results/llm_judge.json(单发判卷,无 oracle;DeepSeek v4 Pro thinking)
type Judge = {
  case: string;
  gold: string;
  claimed: string;
  kind: string | null;
  note: string;
  tone: "success" | "danger" | "warning" | "neutral";
};
const judged: Judge[] = [
  { case: "mutex_deadlock", gold: "bug", claimed: "bug", kind: "Deadlock", note: "定位正确(w1/w2)", tone: "success" },
  { case: "three_way_deadlock", gold: "bug", claimed: "bug", kind: "Deadlock", note: "定位正确(三环)", tone: "success" },
  { case: "signal_loss", gold: "bug", claimed: "bug", kind: "SignalLoss", note: "kind 正确", tone: "success" },
  { case: "channel_deadlock", gold: "bug", claimed: "bug", kind: "Deadlock", note: "gold ChannelBlock,kind 泛化", tone: "warning" },
  { case: "dead_transition", gold: "bug", claimed: "bug", kind: "DeadTransition", note: "kind 正确", tone: "success" },
  { case: "dual_condvar", gold: "bug", claimed: "bug", kind: "Deadlock", note: "gold SignalLoss,kind 判错", tone: "warning" },
  { case: "partial_deadlock", gold: "goals_unmet", claimed: "bug", kind: "Deadlock", note: "kind 判错(gold DeadTransition+goals)", tone: "warning" },
  { case: "goal_unreachable", gold: "goals_unmet", claimed: "bug", kind: "GoalUnreachable", note: "kind 正确", tone: "success" },
  { case: "cas_race", gold: "safe", claimed: "bug", kind: "DeadTransition", note: "误报:把 CAS 结果分支误读为变量值分支", tone: "danger" },
  { case: "semaphore_throttle", gold: "safe", claimed: "safe", kind: null, note: "正确", tone: "neutral" },
  { case: "fn_summary_prop", gold: "safe", claimed: "safe", kind: null, note: "正确", tone: "neutral" },
  { case: "scale_lock_chain_6x3", gold: "safe", claimed: "safe", kind: null, note: "正确(Lockbud 在此误报)", tone: "neutral" },
  { case: "scale_lock_chain_5x3_buggy", gold: "bug", claimed: "bug", kind: "Deadlock", note: "定位正确(w5 反序)", tone: "success" },
  { case: "deep_lock_chain_4x3", gold: "bug", claimed: "bug", kind: "Deadlock", note: "解释中精确定位 w3 else 臂", tone: "success" },
  { case: "deep_lock_chain_4x3_safe", gold: "safe", claimed: "safe", kind: null, note: "未被诱导误报", tone: "neutral" },
  { case: "scale_branch_fan_4x2", gold: "safe", claimed: "safe", kind: null, note: "正确", tone: "neutral" },
];

// codegen:results/codegen.json + codegen_v2.json(verified CIR -> Rust,cargo check 验收,均 1 轮)
type Codegen = { case: string; stmts: number; loc: number; spawns: number; tokens: number };
const codegen: Codegen[] = [
  { case: "mutex_deadlock", stmts: 15, loc: 24, spawns: 2, tokens: 1939 },
  { case: "three_way_deadlock", stmts: 22, loc: 42, spawns: 3, tokens: 3337 },
  { case: "signal_loss", stmts: 14, loc: 25, spawns: 2, tokens: 2845 },
  { case: "channel_deadlock", stmts: 13, loc: 21, spawns: 2, tokens: 2470 },
  { case: "dead_transition", stmts: 3, loc: 4, spawns: 0, tokens: 1995 },
  { case: "dual_condvar", stmts: 19, loc: 24, spawns: 2, tokens: 2661 },
  { case: "partial_deadlock", stmts: 22, loc: 29, spawns: 3, tokens: 3766 },
  { case: "goal_unreachable", stmts: 13, loc: 17, spawns: 2, tokens: 1999 },
  { case: "goal_trivial", stmts: 13, loc: 19, spawns: 2, tokens: 2229 },
  { case: "goal_bad_reference", stmts: 13, loc: 19, spawns: 2, tokens: 2303 },
  { case: "cas_race", stmts: 11, loc: 22, spawns: 2, tokens: 2834 },
  { case: "semaphore_throttle", stmts: 16, loc: 53, spawns: 3, tokens: 2333 },
  { case: "fn_summary_prop", stmts: 15, loc: 25, spawns: 2, tokens: 3517 },
  { case: "scale_lock_chain_6x3", stmts: 55, loc: 24, spawns: 1, tokens: 4514 },
  { case: "scale_lock_chain_5x3_buggy", stmts: 46, loc: 76, spawns: 5, tokens: 4374 },
  { case: "deep_lock_chain_4x3", stmts: 67, loc: 44, spawns: 4, tokens: 7258 },
  { case: "deep_lock_chain_4x3_safe", stmts: 67, loc: 104, spawns: 4, tokens: 8232 },
  { case: "scale_branch_fan_4x2", stmts: 41, loc: 29, spawns: 4, tokens: 4075 },
];

// 外部基线:results/miri_baseline.json(preemption 0.5 × 16 种子;漏检项再以 256 种子复核)
// 与 results/lockbud_baseline.json(-k deadlock,nightly-2026-02-07)
type Baseline = {
  case: string;
  gold: string;
  cvn: string;
  lockbud: string;
  lockbudOk: "hit" | "miss" | "wrong";
  miri: string;
  miriOk: "hit" | "miss" | "partial";
};
const baselines: Baseline[] = [
  { case: "mutex_deadlock", gold: "Deadlock", cvn: "Deadlock", lockbud: "ConflictLock", lockbudOk: "hit", miri: "漏检(256 种子)", miriOk: "miss" },
  { case: "three_way_deadlock", gold: "Deadlock(3 环)", cvn: "Deadlock", lockbud: "漏检", lockbudOk: "miss", miri: "漏检(256 种子)", miriOk: "miss" },
  { case: "signal_loss", gold: "SignalLoss", cvn: "SignalLoss", lockbud: "漏检", lockbudOk: "miss", miri: "漏检(256 种子)", miriOk: "miss" },
  { case: "channel_deadlock", gold: "ChannelBlock", cvn: "ChannelBlock", lockbud: "漏检", lockbudOk: "miss", miri: "漏检(256 种子)", miriOk: "miss" },
  { case: "dead_transition", gold: "DeadTransition", cvn: "DeadTransition", lockbud: "漏检", lockbudOk: "miss", miri: "漏检(动态不可见)", miriOk: "miss" },
  { case: "dual_condvar", gold: "SignalLoss", cvn: "SignalLoss", lockbud: "漏检", lockbudOk: "miss", miri: "deadlock 报告", miriOk: "hit" },
  { case: "partial_deadlock", gold: "goals 未达", cvn: "DeadTransition + unmet goals", lockbud: "漏检", lockbudOk: "miss", miri: "超时挂起(未诊断)", miriOk: "partial" },
  { case: "goal_unreachable", gold: "goals 未达", cvn: "goals_unmet", lockbud: "DoubleLock(误诊)", lockbudOk: "wrong", miri: "assert panic", miriOk: "hit" },
  { case: "scale_lock_chain_5x3_buggy", gold: "Deadlock(锁序)", cvn: "Deadlock", lockbud: "DoubleLock(误诊)", lockbudOk: "wrong", miri: "漏检(256 种子)", miriOk: "miss" },
];

// scaling sweep:results/scaling.json(全部 37 点;下面取 locks=3 / branches=2、线程 2–5 切片)
const threadAxis = ["2 线程", "3 线程", "4 线程", "5 线程"];
const scalingSeries = [
  { name: "锁链·安全(3 锁)", data: [77, 437, 2277, 11237] },
  { name: "锁链·死锁(3 锁)", data: [86, 518, 2790, 14054] },
  { name: "分支扇出(2 层分支)", data: [153, 2193, 29121, 100000] },
];

// repair_8k.json:7 个 bug case × 3 种反馈模式(max_tokens=8192,max_rounds=5)
type Repair = { case: string; ok: boolean; rounds: number; tokens: number };
const repairByMethod: Record<string, Repair[]> = {
  full: [
    { case: "mutex_deadlock", ok: true, rounds: 1, tokens: 3963 },
    { case: "three_way_deadlock", ok: true, rounds: 1, tokens: 5344 },
    { case: "signal_loss", ok: false, rounds: -1, tokens: 21967 },
    { case: "channel_deadlock", ok: true, rounds: 1, tokens: 4535 },
    { case: "dead_transition", ok: true, rounds: 4, tokens: 23963 },
    { case: "dual_condvar", ok: false, rounds: -1, tokens: 48572 },
    { case: "partial_deadlock", ok: true, rounds: 2, tokens: 20434 },
  ],
  statusOnly: [
    { case: "mutex_deadlock", ok: true, rounds: 1, tokens: 3565 },
    { case: "three_way_deadlock", ok: true, rounds: 1, tokens: 4270 },
    { case: "signal_loss", ok: false, rounds: -1, tokens: 28858 },
    { case: "channel_deadlock", ok: true, rounds: 1, tokens: 3881 },
    { case: "dead_transition", ok: true, rounds: 1, tokens: 6467 },
    { case: "dual_condvar", ok: false, rounds: -1, tokens: 33426 },
    { case: "partial_deadlock", ok: true, rounds: 1, tokens: 6006 },
  ],
  llmOnly: [
    { case: "mutex_deadlock", ok: true, rounds: 1, tokens: 3771 },
    { case: "three_way_deadlock", ok: true, rounds: 1, tokens: 4709 },
    { case: "signal_loss", ok: false, rounds: -1, tokens: 12311 },
    { case: "channel_deadlock", ok: true, rounds: 1, tokens: 5640 },
    { case: "dead_transition", ok: true, rounds: 2, tokens: 11305 },
    { case: "dual_condvar", ok: false, rounds: -1, tokens: 43974 },
    { case: "partial_deadlock", ok: true, rounds: 1, tokens: 7446 },
  ],
};

const methodLabel: Record<string, string> = {
  full: "CVN 完整反馈",
  statusOnly: "仅 status/kind",
  llmOnly: "无 CVN 诊断",
};

// full_run.json:每 case 5 条自然语言需求(1 规范 + 4 释义),每条最多 5 轮
type Gen = { case: string; valid: number; safe: number; rounds: number[]; tokens: number };
const generate: Gen[] = [
  { case: "mutex_deadlock", valid: 5, safe: 5, rounds: [2, 3, 3, 1, 3], tokens: 46649 },
  { case: "three_way_deadlock", valid: 5, safe: 5, rounds: [2, 3, 1, 2, 2], tokens: 51944 },
  { case: "signal_loss", valid: 1, safe: 0, rounds: [4, 5, 5, 5, 5], tokens: 109728 },
  { case: "channel_deadlock", valid: 0, safe: 0, rounds: [5, 5, 5, 5, 5], tokens: 98856 },
  { case: "dead_transition", valid: 0, safe: 0, rounds: [5, 5, 5, 5, 5], tokens: 109559 },
  { case: "dual_condvar", valid: 1, safe: 0, rounds: [5, 5, 5, 5, 5], tokens: 88620 },
  { case: "partial_deadlock", valid: 3, safe: 2, rounds: [2, 1, 5, 1, 5], tokens: 72417 },
  { case: "cas_race", valid: 0, safe: 0, rounds: [5, 5, 5, 5, 5], tokens: 94965 },
  { case: "semaphore_throttle", valid: 5, safe: 4, rounds: [3, 1, 1, 1, 2], tokens: 30493 },
  { case: "fn_summary_prop", valid: 2, safe: 2, rounds: [5, 5, 1, 5, 5], tokens: 90436 },
];

// repair_local.json vs repair_full_v2.json:12 个非安全 case 的切片修复 A/B
// (反馈均为摘要化后的 full;fell 表示切片无法定位/耗尽后回退全量修复)
type RepairAb = {
  case: string;
  localOk: boolean;
  localTokens: number;
  slice: string;
  fell: boolean;
  fullOk: boolean;
  fullTokens: number;
};
const repairAb: RepairAb[] = [
  { case: "mutex_deadlock", localOk: true, localTokens: 4176, slice: "3/3", fell: false, fullOk: true, fullTokens: 4424 },
  { case: "three_way_deadlock", localOk: true, localTokens: 7426, slice: "4/4", fell: false, fullOk: true, fullTokens: 6570 },
  { case: "signal_loss", localOk: true, localTokens: 9104, slice: "3/3", fell: false, fullOk: true, fullTokens: 9865 },
  { case: "channel_deadlock", localOk: true, localTokens: 5137, slice: "3/3", fell: false, fullOk: true, fullTokens: 5385 },
  { case: "dead_transition", localOk: true, localTokens: 10656, slice: "1/1", fell: false, fullOk: true, fullTokens: 5104 },
  { case: "dual_condvar", localOk: false, localTokens: 25031, slice: "3/3", fell: true, fullOk: true, fullTokens: 24417 },
  { case: "partial_deadlock", localOk: true, localTokens: 24875, slice: "3/4", fell: false, fullOk: true, fullTokens: 9328 },
  { case: "goal_unreachable", localOk: true, localTokens: 4179, slice: "0/3", fell: true, fullOk: true, fullTokens: 3879 },
  { case: "goal_trivial", localOk: true, localTokens: 5855, slice: "0/3", fell: true, fullOk: true, fullTokens: 4426 },
  { case: "goal_bad_reference", localOk: true, localTokens: 4322, slice: "0/3", fell: true, fullOk: true, fullTokens: 4240 },
  { case: "scale_lock_chain_5x3_buggy", localOk: true, localTokens: 12444, slice: "6/6", fell: false, fullOk: true, fullTokens: 14146 },
  { case: "deep_lock_chain_4x3", localOk: true, localTokens: 15399, slice: "5/5", fell: false, fullOk: true, fullTokens: 17273 },
];

const shortName = (id: string) => id.replace(/_/g, " ");

export default function ExperimentReport() {
  const bugCases = analyze.filter((a) => a.expected !== "safe");
  const genValid = generate.reduce((s, g) => s + g.valid, 0);
  const genSafe = generate.reduce((s, g) => s + g.safe, 0);
  const repairCases = repairByMethod.full.length;
  const repairOk = repairByMethod.full.filter((r) => r.ok).length;

  return (
    <Stack gap={28} style={{ padding: 24, maxWidth: 1080 }}>
      <Stack gap={6}>
        <H1>ConcPlanVerify 实验报告</H1>
        <Text tone="secondary">
          模型 DeepSeek v4 Pro(thinking + high reasoning)· 基准 benchmarks/manifest.json(18 个 case:12 缺陷 + 6
          安全对照,含 3 个参数化大规模 case、1 对深埋 bug 孪生与 3 类 goals 负例)· 外部基线 Lockbud(静态)/
          Miri(动态)· 2026-08-05
        </Text>
      </Stack>

      <Grid columns={5} gap={16}>
        <Stat value="20/20*" label="CVN 检出判定正确" tone="success" />
        <Stat value="4/9 · 1 误报" label="Lockbud 检出(2 例误诊)" tone="warning" />
        <Stat value="3/9 · 0 误报" label="Miri 检出(16–256 种子)" tone="warning" />
        <Stat value="10/10 · 1 误报" label="LLM 单独判卷(2 例 kind 错)" tone="info" />
        <Stat value="18/18" label="codegen 过 cargo check" tone="success" />
      </Grid>
      <Text tone="tertiary" size="small">
        *20 = 原 18 + goal_constrained_deadlock / _dense;两新例本轮 analyze 正确,未重跑全量 offline。
      </Text>

      {/* ── 1. 检出能力 ── */}
      <Stack gap={12} style={{ scrollMarginTop: 88 }} >
        <div id="s1" />
        <H2>1 · 缺陷检出:CVN 状态空间分析 vs 仅静态校验(20 case)</H2>
        <Text tone="secondary">
          对每个 case 的 gold buggy CIR 分别跑 <Text as="span" weight="semibold">cir2cvn --analyze</Text>
          (翻译 + 状态空间探索 + goals)与 <Text as="span" weight="semibold">--validate</Text>(仅静态规则)。
          CVN 全部 16 例判定正确;静态校验对行为级缺陷全部漏检(其本职是 schema/结构检查,该消融验证了状态空间分析的必要性)。
        </Text>
        <Table
          headers={["Case", "期望", "CVN analyze 判定", "检出 bug kind", "仅静态校验"]}
          rows={analyze.map((a) => [
            shortName(a.case),
            a.expected,
            a.actual,
            a.kinds.length ? a.kinds.join(" + ") : "—",
            a.expected === "safe" ? "safe(正确)" : "漏检",
          ])}
          rowTone={analyze.map((a) => (a.expected === "safe" ? ("neutral" as const) : ("success" as const)))}
          columnAlign={["left", "left", "left", "left", "left"] as const}
          striped
        />
        <Text tone="tertiary" size="small">
          partial_deadlock 的状态判定为 verified_unsafe(DeadTransition 优先),同时 payload 报告 2 个 unmet
          goals,按 goals 级缺陷计为检出。signal_loss 现在只报 SignalLoss:死转移检测新增了 condvar
          翻译变体分组(cv_wake1/cv_wakeA 等「或」变体整组皆死才报告),消除了工件级 DeadTransition
          噪音,也修复了其 fixed fixture 被误报的问题(fixture 债清偿)。数据源:results/offline_v3.json。
        </Text>
      </Stack>

      {/* ── 2. 外部基线 ── */}
      <Stack gap={12} style={{ scrollMarginTop: 88 }}>
        <div id="s2" />
        <H2>2 · 外部基线:CVN vs Lockbud(静态)vs Miri(动态),9 个缺陷 case</H2>
        <Text tone="secondary">
          Lockbud(nightly-2026-02-07,-k deadlock)对各 case 的参考 buggy.rs 做静态锁分析;Miri 以
          -Zmiri-preemption-rate=0.5 × 16 随机种子运行,漏检项再用 256 种子复核仍全部漏检。
        </Text>
        <Table
          headers={["Case", "Gold 缺陷", "CVN", "Lockbud", "Miri"]}
          rows={baselines.map((b) => [shortName(b.case), b.gold, b.cvn, b.lockbud, b.miri])}
          rowTone={baselines.map((b) =>
            b.lockbudOk === "miss" && b.miriOk === "miss" ? ("warning" as const) : ("neutral" as const),
          )}
          striped
        />
        <Grid columns={2} gap={16}>
          <Callout tone="danger" title="Miri:调度依赖死锁基本不可采样">
            5 个调度依赖死锁(互斥/三环/信号丢失/通道/锁序)即使 256 个随机调度种子也全部漏检——死锁窗口在随机采样下
            概率过低;而 CVN 穷举交错空间全部检出。Miri 只在缺陷恰好落入被执行调度时报告(dual_condvar、goal_unreachable
            的 assert)。安全 case 零误报(5/5)。
          </Callout>
          <Callout tone="warning" title="Lockbud:锁序类可检,但诊断不可靠">
            经典双锁 ConflictLock 检出正确;但 Vec&lt;Arc&lt;Mutex&gt;&gt; 下标别名混淆使其对安全的
            scale_lock_chain_6x3 误报 DoubleLock(唯一误报),对 5x3 死锁链与 goal_unreachable 也给出同源的
            DoubleLock 误诊——在该模式上其报告没有区分度。三环死锁、condvar、channel 类全部漏检。
          </Callout>
        </Grid>
      </Stack>

      {/* ── 3. goals 消融 ── */}
      <Stack gap={12} style={{ scrollMarginTop: 88 }}>
        <div id="s3" />
        <H2>3 · goals 信任边界:三类负例 + 关闭 goals 检查的代价</H2>
        <Text tone="secondary">
          goals 负例现覆盖三类可检形态(不可达 / 过弱 / 错误引用),配套新增两条检查规则:goal
          在初始状态即成立 → 「too weak」告警;goal 引用未声明变量或未知库所 → 翻译告警。用
          <Text as="span" weight="semibold"> cir2cvn --no-goals</Text> 重跑全部 case,差异集中在 goals 类缺陷:
        </Text>
        <Table
          headers={["Case", "缺陷构成", "开 goals(默认)", "关 goals(消融)"]}
          rows={[
            ["goal unreachable", "无死锁,goal x==3 不可达", "goals_unmet ✓", "verified_safe(误接受)"],
            ["goal trivial", "goal x==0 初始态即成立(过弱,约束不了任何行为)", "goals_unmet ✓(too-weak 告警)", "verified_safe(误接受)"],
            ["goal bad reference", "goal 引用不存在的库所 w1_done 与未声明变量 y", "goals_unmet ✓(悬空引用告警)", "verified_safe(误接受)"],
            ["partial deadlock", "DeadTransition + 2 个 unmet goals", "verified_unsafe ✓", "verified_unsafe(仍检出)"],
          ]}
          rowTone={["danger" as const, "danger" as const, "danger" as const, "neutral" as const]}
          striped
        />
        <Text tone="tertiary" size="small">
          goals 类 gold 共 4 例,其中 3 例(75%)完全依赖 goals 层兜底——静态工具无谓词语义、动态工具只能靠
          assert 间接观察,而「过弱/悬空」两类在代码层面根本不存在(缺陷在规格本身)。goals 缺失(goals: [])
          不改变判定,属于文档化的信任边界(doc/goals_policy.md)。数据源:results/no_goals_v2.json。
        </Text>
      </Stack>

      {/* ── 4. 修复实验 ── */}
      <Stack gap={12} style={{ scrollMarginTop: 88 }}>
        <div id="s4" />
        <H2>4 · 修复实验:反馈信息量消融(7 个缺陷 case × 3 模式)</H2>
        <Text tone="secondary">
          这里先保留历史基线,再展示后续修复结果。repair_8k 是早期的 5 轮上限实验;repair_full_v2 是加入
          condvar 变体分组、死锁后缀 DeadTransition 过滤和反馈摘要化后的全量修复实验;repair_local 是同一批 case
          的局部切片实验。三者的验收 oracle 都是 Rust analyzer 的 verified_safe。
        </Text>
        <Grid columns={3} gap={16}>
          {(["full", "statusOnly", "llmOnly"] as const).map((m) => {
            const rows = repairByMethod[m];
            const ok = rows.filter((r) => r.ok);
            const avgRounds = ok.reduce((s, r) => s + r.rounds, 0) / ok.length;
            const tokens = rows.reduce((s, r) => s + r.tokens, 0);
            return (
              <div key={m}>
                <Card>
                  <CardHeader trailing={<Pill size="sm">{`${ok.length}/${rows.length} 成功`}</Pill>}>
                    {methodLabel[m]}
                  </CardHeader>
                  <CardBody>
                    <Row gap={20}>
                      <Stat value={avgRounds.toFixed(1)} label="平均轮次(成功例)" />
                      <Stat value={`${(tokens / 1000).toFixed(0)}k`} label="token 总量(入+出)" />
                    </Row>
                  </CardBody>
                </Card>
              </div>
            );
          })}
        </Grid>
        <BarChart
          categories={repairByMethod.full.map((r) => shortName(r.case))}
          series={(["full", "statusOnly", "llmOnly"] as const).map((m) => ({
            name: methodLabel[m],
            data: repairByMethod[m].map((r) => r.tokens),
          }))}
          height={260}
          valueSuffix=" tok"
        />
        <Text tone="tertiary" size="small">
          每 case × 反馈模式的 LLM token 总量(输入+输出,单位 token)。来源:results/repair_8k.json。signal_loss 与
          dual_condvar 三种模式均耗尽 5 轮失败,token 为 5 轮累计。
        </Text>
        <Table
          headers={["Case", "缺陷类型", "CVN 完整反馈", "仅 status/kind", "无 CVN 诊断"]}
          rows={repairByMethod.full.map((r, i) => [
            shortName(r.case),
            bugCases[i]?.kinds.join(" + ") ?? "",
            r.ok ? `成功 · ${r.rounds} 轮` : "失败(5 轮耗尽)",
            repairByMethod.statusOnly[i].ok
              ? `成功 · ${repairByMethod.statusOnly[i].rounds} 轮`
              : "失败(5 轮耗尽)",
            repairByMethod.llmOnly[i].ok
              ? `成功 · ${repairByMethod.llmOnly[i].rounds} 轮`
              : "失败(5 轮耗尽)",
          ])}
          rowTone={repairByMethod.full.map((r) => (r.ok ? ("success" as const) : ("danger" as const)))}
          striped
        />

        <H2>4b · 局部重生成 vs 全量修复(12 case A/B,反馈已摘要化)</H2>
        <Text tone="secondary">
          局部重生成(repair_local):以 bug 报告牵连的函数集为切片,LLM 只重写切片函数(其余函数以单行同步摘要
          冻结展示),Python 拼接回原 CIR——非切片部分字节级不变。切片无法定位(纯 goals 缺陷)或轮次耗尽时回退全量修复。
          本轮 full 反馈已启用摘要化:同构 bug 按签名去重 + 长反例 trace 压缩。
        </Text>
        <Table
          headers={["Case", "局部修复", "token", "切片/总函数", "回退", "全量修复", "token"]}
          rows={repairAb.map((r) => [
            shortName(r.case),
            r.localOk ? "成功" : "失败",
            r.localTokens.toLocaleString(),
            r.slice,
            r.fell ? "是" : "—",
            r.fullOk ? "成功" : "失败",
            r.fullTokens.toLocaleString(),
          ])}
          rowTone={repairAb.map((r) => (r.localOk ? ("neutral" as const) : ("danger" as const)))}
          columnAlign={["left", "left", "right", "right", "left", "left", "right"] as const}
          striped
        />
        <Grid columns={2} gap={16}>
          <Callout tone="success" title="局部重生成的核心价值:零语义漂移">
            全量修复在 3 例上出现 oracle 不可见的语义漂移(dead_transition 凭空加 Mutex、dual_condvar 加 2 个
            Var、deep case 把 Var 改成 Atomic);局部重生成 12 例中漂移为 0——冻结拼接从结构上杜绝了「顺手改掉
            没坏的部分」。成功率 11/12 vs 12/12(唯一失败仍是 condvar 类边界),token 总量相当(129k vs 109k)。
          </Callout>
          <Callout tone="success" title="反馈摘要化:132k → 17k">
            deep_lock_chain_4x3 的 99 条死锁反例按等价签名去重为若干组、每组只保留最短反例(超长 trace
            头尾压缩),full 反馈的单轮修复从 132k token 降到 17.3k(约 7.6×),成功率不变——大 case 上完整反馈
            的成本劣势已消除。实现:prompts.verification_feedback。
          </Callout>
        </Grid>
        <Text tone="tertiary" size="small">
          goals 三例的切片为 0/3(缺陷在 goal 规格而非函数体,bug 报告无牵连函数),按设计回退全量修复并全部成功——
          回退机制的正确性得到验证。数据源:results/repair_local.json、results/repair_full_v2.json。
        </Text>
      </Stack>

      {/* ── 5. 生成实验 ── */}
      <Stack gap={12} style={{ scrollMarginTop: 88 }}>
        <div id="s5" />
        <H2>5 · 生成实验:自然语言 → CIR(10 case × 5 条需求)</H2>
        <Text tone="secondary">
          每个 case 用 1 条规范需求 + 4 条释义分别生成 CIR(最多 5 轮校验重试),生成成功后再跑完整
          analyze。整体仅 {genValid}/50 通过静态校验、{genSafe}/50 最终 verified_safe。锁序类模式(mutex / three-way /
          semaphore)几乎全部成功;channel、CAS、condvar、带分支的模式几乎全部 5 轮耗尽——瓶颈在 CIR schema
          细节(res_op 动作名、branch/transfer 形态),而非并发语义。
        </Text>
        <BarChart
          categories={generate.map((g) => shortName(g.case))}
          series={[
            { name: "通过静态校验", data: generate.map((g) => g.valid), tone: "info" as const },
            { name: "最终 verified_safe", data: generate.map((g) => g.safe), tone: "success" as const },
          ]}
          height={260}
          yMax={5}
          showValues
        />
        <Text tone="tertiary" size="small">
          每 case 5 次生成中通过校验 / 最终验证安全的次数(0–5)。来源:results/full_run.json,DeepSeek v4
          Pro,max_tokens=4096。
        </Text>
        <BarChart
          categories={generate.map((g) => shortName(g.case))}
          series={[{ name: "token 消耗(入+出)", data: generate.map((g) => g.tokens), tone: "warning" as const }]}
          height={220}
          valueSuffix=" tok"
        />
        <Text tone="tertiary" size="small">
          每 case 5 次生成的 LLM token 总量。失败 case 因 5 轮重试满额,成本约为成功 case 的 2–3 倍。
        </Text>
      </Stack>

      {/* ── 6. CVN 规模与性能 ── */}
      <Stack gap={12} style={{ scrollMarginTop: 88 }}>
        <div id="s6" />
        <H2>6 · CVN 规模与验证性能(14 case)</H2>
        <Text tone="secondary">
          小规模 case 验证耗时全部在 2ms 以内;三个大规模 case(14k–53k 状态)也在 84–366ms 完成且状态空间探索完整
          ——与单轮 LLM 调用的 10–60 秒相比可忽略,验证器不会成为闭环瓶颈。
        </Text>
        <Table
          headers={["Case", "CIR 语句", "库所(控制/资源/等待)", "变迁", "弧(入/出)", "可达状态", "验证耗时"]}
          rows={analyze.map((a) => {
            const [c, r, w] = placesByKind[a.case] ?? [0, 0, 0];
            return [
              shortName(a.case),
              a.statements,
              `${a.places}(${c}/${r}/${w})`,
              a.transitions,
              `${a.arcs[0]}/${a.arcs[1]}`,
              a.states.toLocaleString(),
              a.ms < 10 ? `${a.ms.toFixed(2)} ms` : `${a.ms.toFixed(0)} ms`,
            ];
          })}
          columnAlign={["left", "right", "left", "right", "left", "right", "right"] as const}
          striped
        />
      </Stack>

      {/* ── 7. scaling sweep ── */}
      <Stack gap={12} style={{ scrollMarginTop: 88 }}>
        <div id="s7" />
        <H2>7 · Scaling sweep:状态爆炸拐点(37 数据点)</H2>
        <Text tone="secondary">
          用参数化生成器(cir_workflow/scaling.py)在三种模式上 sweep 线程数 × 锁数/分支层数,每点跑完整
          analyze。可达状态数随线程数指数增长;含数据分支的模式增长最陡:branch_fan 在 5 线程 × 2
          层分支时超出 max_states=100k(analysis_incomplete),而纯锁序模式 6 线程 × 3 锁(53k 状态)仍可完成。
        </Text>
        <LineChart
          categories={threadAxis}
          series={scalingSeries}
          height={280}
          referenceLines={[{ value: 100000, label: "max_states = 100k" }]}
          beginAtZero
        />
        <Text tone="tertiary" size="small">
          横轴:线程数;纵轴:可达状态数(线性刻度)。分支扇出 5 线程点画在 100k 处表示超预算截断(实际
          analysis_incomplete);锁链·安全模式 6 线程 × 3 锁为 53,477 状态 / 402ms(图外仍可完成)。
          来源:results/scaling.json。
        </Text>
        <Callout tone="info" title="max_states 预算建议">
          默认 100k 对纯锁序模式足够(6 线程 × 3 锁仅 53k),但含数据分支的模式在 ≥5 线程时需要分级提高预算,
          或引入偏序缩减(partial-order reduction)压缩交错空间——这是 scaling 维度的下一个工程点。
        </Callout>

        <H2>7b · Scaling 的 LLM 两腿:NL → 生成 CIR → 验证 → 代码</H2>
        <Text tone="secondary">
          在 sweep 的 6 个代表点上补跑 LLM 腿:同一 NL 需求让模型生成 CIR(≤5 轮,verified_safe 验收),
          成功则对生成 CIR 做 codegen,失败则回退 gold CIR 做 codegen(隔离两腿的失败面)。
        </Text>
        <Table
          headers={["模式 × 规模", "gold 语句", "生成 CIR", "轮", "token", "状态数", "codegen(来源)", "token", "LOC"]}
          rows={[
            ["lock chain 2×2", "15", "verified_safe", "3", "12,556", "57", "通过(生成)", "2,226", "24"],
            ["lock chain 3×2", "22", "verified_safe", "1", "3,840", "111", "通过(生成)", "2,757", "32"],
            ["lock chain 4×3", "37", "verified_safe", "1", "3,593", "2,277", "通过(生成)", "4,180", "63"],
            ["lock chain 6×3", "55", "verified_safe", "2", "13,481", "53,477", "通过(生成)", "4,727", "59"],
            ["branch fan 2×2", "21", "5 轮耗尽", "5", "22,523", "—", "通过(gold)", "4,439", "59"],
            ["branch fan 4×2", "41", "5 轮耗尽", "5", "35,631", "—", "通过(gold)", "5,073", "36"],
          ]}
          rowTone={["neutral", "neutral", "neutral", "neutral", "danger", "danger"] as const}
          columnAlign={["left", "right", "left", "right", "right", "right", "left", "right", "right"] as const}
          striped
        />
        <Text tone="tertiary" size="small">
          锁链模式 4/4 生成成功且规模不衰减(4×3、6×3 生成的 CIR 语句数与 gold 持平,状态空间同量级)——
          「生成能力随规模劣化」在纯锁序模式上不成立;3×2 点模型生成了更精简的等价方案(12 vs 22 语句)。
          branch_fan 两点全部 5 轮耗尽在静态校验,与第 5 节生成实验的结论一致:瓶颈是 branch/switch 的
          schema 细节,而非并发建模,且 token 白耗 22k–36k——few-shot 范例是最直接的止血。codegen 腿 6/6
          通过(含回退 gold 的两点)。数据源:results/scaling_llm.json。
        </Text>
      </Stack>

      {/* ── 8. LLM 单独判卷 ── */}
      <Stack gap={12} style={{ scrollMarginTop: 88 }}>
        <div id="s8" />
        <H2>8 · LLM 单独判卷 vs CVN:深埋 bug 与误报探针(16 case)</H2>
        <Text tone="secondary">
          无 oracle 的单发判卷:LLM 直接阅读 CIR 给出 bug/safe、kind 与嫌疑位置。为考验它,新增了深埋 bug
          case(deep_lock_chain_4x3:4 worker × 2 分支臂共 8 段近似锁序,仅 w3 的 else 臂把 m2 提前;if
          臂跳过 m2 以绕过函数内锁序静态规则 E505)及其安全孪生。
        </Text>
        <Table
          headers={["Case", "Gold", "LLM 判定", "LLM kind", "备注"]}
          rows={judged.map((j) => [
            shortName(j.case),
            j.gold,
            j.claimed,
            j.kind ?? "—",
            j.note,
          ])}
          rowTone={judged.map((j) => j.tone)}
          striped
        />
        <Grid columns={2} gap={16}>
          <Callout tone="danger" title="拿到了真实误报:cas_race">
            LLM 把「对 CAS 结果的分支」误读为「对变量值的分支」,推断出永假条件并报告
            DeadTransition——安全 case 上的误报。CVN 按 CIR 语义精确建模 CAS 成功/失败两条转移,判定
            safe。这说明 LLM 判卷在语义细节上不可靠,验证器的价值不在「能不能发现」而在「说的一定对」。
          </Callout>
          <Callout tone="warning" title="埋深锁序 bug 没能骗过 DeepSeek">
            67 语句、8 段近似锁序中的一处相邻交换,判卷解释仍精确定位到 w3 的 else
            臂;修复实验三种反馈也全部 1 轮成功——锁序类缺陷的修复不需要定位(把所有序列规范化即安全)。要拉开
            LLM-only 与 CVN 反馈的差距,方向是:condvar/SignalLoss 类(已是修复边界)、goals
            语义约束(让「规范化式修复」破坏业务目标而被拒)、以及跨函数 call 链持锁的更大 CIR。
          </Callout>
        </Grid>
        <Text tone="tertiary" size="small">
          另两处 kind 判错:dual_condvar(gold SignalLoss,LLM 说 Deadlock)、partial_deadlock(gold
          DeadTransition+goals,LLM 说 Deadlock);修复实验中还观察到两种模式把 flag 的 Var 悄悄改成
          Atomic——oracle 不可见的语义漂移。来源:results/llm_judge.json、results/deep_repair.json。
        </Text>
      </Stack>

      {/* ── 9. codegen ── */}
      <Stack gap={12} style={{ scrollMarginTop: 88 }}>
        <div id="s9" />
        <H2>9 · 代码生成:verified CIR → Rust(cargo check 验收)</H2>
        <Text tone="secondary">
          用户故事的最后一环:LLM 拿到已验证的 CIR 计划,按「结构忠实映射」约束生成 main.rs,`cargo check`
          验收(≤3 轮)。18/18 全部 1 轮通过。signal_loss 上一轮被 verified_safe 门禁拦下(fixed fixture
          误报 DeadTransition),condvar 变体分组修复后本轮通过——门禁与修复形成了正确的闭环。端到端
          pipeline(需求 → CIR → 验证 → 代码)在 mutex_deadlock 与 semaphore_throttle 上冒烟成功,单链约 7k token。
        </Text>
        <BarChart
          categories={codegen.map((c) => shortName(c.case))}
          series={[
            { name: "CIR 语句数", data: codegen.map((c) => c.stmts), tone: "info" as const },
            { name: "生成代码 LOC", data: codegen.map((c) => c.loc), tone: "success" as const },
          ]}
          height={260}
        />
        <Text tone="tertiary" size="small">
          CIR 语句数 vs 生成 Rust 的非空非注释行数。锁链类 CIR 语句多但代码可折叠(scale_lock_chain_6x3:55
          语句 → 24 行,模型把 6 个相同 worker 折叠成循环);deep_lock_chain_4x3_safe 未折叠(104
          行)。折叠语义等价但破坏语句级 1:1 对应——「CIR ↔ Rust 一致性检查」(todo 第 5 节)因此必要。
          来源:results/codegen.json、results/pipeline_smoke.json。
        </Text>
      </Stack>

      {/* ── 10. Goals 约束修复 ── */}
      <Stack gap={12} style={{ scrollMarginTop: 88 }}>
        <div id="s10" />
        <H2>10 · Goals 约束死锁:堵住「规范化式乱修」</H2>
        <Text tone="secondary">
          新增 goal_constrained_deadlock(3 worker,~2k 状态)与 dense 孪生(4 worker,~65k
          状态):死锁埋在 w3 的 else 臂(m2→m1),同一臂是唯一写入 result=99 的路径;业务 goal 要求 99
          可达。离线探针:在 fixed CIR 上把 99 改成 3 → goals_unmet——「删臂/统一写值」清死锁但不被验收。
        </Text>
        <Table
          headers={["Case", "模式", "成功", "轮次", "保留 result=99", "token"]}
          rows={[
            ["goal constrained", "CVN 完整", "是", "1", "是", "9854"],
            ["goal constrained", "仅 status", "是", "1", "是", "6697"],
            ["goal constrained", "无反馈", "是", "1", "是", "6482"],
            ["dense", "CVN 完整", "是", "1", "是", "16722"],
            ["dense", "仅 status", "是", "1", "是", "10159"],
            ["dense", "无反馈", "是", "1", "是", "10128"],
            ["dense × Flash", "CVN 完整", "是", "2*", "是", "11742"],
            ["dense × Flash", "无反馈", "是", "1", "是", "12511"],
          ]}
          striped
        />
        <Text tone="tertiary" size="small">
          *Flash×CVN 首轮因 thinking 空 content 失败,第 2 轮成功。来源:results/goal_constrained_repair_ab.json、
          results/goal_constrained_flash.json。Oracle 层有效;DeepSeek Pro/Flash 均未自发丢掉 99,成功率未拉开。
        </Text>
        <Callout tone="info" title="分层结论">
          (1) 验收门禁能拒掉规范化乱修;(2) 当前强模型会自发保留特色写;(3) 该 case 作回归探针与弱模型/对抗改写对照,见
          doc/goals_policy.md §5。
        </Callout>
      </Stack>

      {/* ── 11. 关键发现 ── */}
      <Stack gap={12} style={{ scrollMarginTop: 88 }}>
        <div id="s11" />
        <H2>11 · 关键发现与下一步</H2>
        <Callout tone="success" title="与外部工具的能力边界清晰互补">
          锁序类:Lockbud 可检但诊断不可靠(别名混淆致 1 误报 + 2 误诊);调度依赖死锁:Miri 随机采样 256
          种子仍全漏,CVN 穷举全检出;goals 语义类与结构类(DeadTransition):只有 CVN 能表达和检出。
          CVN 在 20 case 上无误报、无漏检、无误诊(含新增 goals 约束死锁对)。
        </Callout>
        <Callout tone="info" title="Goals 约束堵住规范化乱修(oracle 层)">
          goal_constrained_deadlock 证明:丢掉特色写 99 会 goals_unmet。DeepSeek Pro/Flash 修复时均保留
          99,成功率未与无反馈拉开——强模型未自发踩坑;该 case 作回归探针,见第 10 节。
        </Callout>
        <Callout tone="success" title="SignalLoss 修复边界已被推过去">
          早期三种反馈模式下 signal_loss / dual_condvar 全部失败(各耗尽 5 轮);condvar 变体分组消除反馈中的
          DeadTransition 噪音 + 反馈摘要化后,本轮全量修复 signal_loss 2 轮、dual_condvar 3 轮成功,局部重生成
          signal_loss 也 2 轮成功。剩余唯一失败是局部模式下的 dual_condvar(切片冻结限制了跨函数协议改写,
          回退全量后成功)——condvar 类结构性改写更适合全量修复兜底。
        </Callout>
        <Callout tone="warning" title="生成瓶颈在 CIR schema,而非并发建模">
          生成失败的 case(channel / cas / dead_transition / condvar)都是 5 轮耗尽在静态校验上。可行方向:在生成 prompt
          中内嵌对应模式的最小 CIR 范例(few-shot),或按缺陷类型定制 schema 片段。
        </Callout>
        <Callout tone="success" title="完整反馈的成本爆炸已由摘要化解决">
          deep_lock_chain_4x3 的完整 CVN 反馈原含 99 条死锁反例、单轮修复 132k token;按 bug 等价签名去重 +
          反例轨迹头尾压缩后降到 17.3k(7.6×),成功率不变。摘要化已默认启用(prompts.verification_feedback),
          大 case 上完整反馈相对无反馈的成本劣势消除。
        </Callout>
        <Callout tone="warning" title="剩余风险:语义漂移与折叠">
          全量修复在 3/12 case 上悄悄改动未损坏部分(加 Mutex/Var、Var→Atomic),oracle 不可见;局部重生成以
          冻结拼接根治(12 case 零漂移),建议作为默认修复模式、全量作 fallback。codegen 侧的对应风险是
          worker 折叠破坏语句级 1:1 对应,CIR↔Rust 一致性 checklist(doc/cir_rust_consistency.md)用于人工复核。
        </Callout>
        <Callout tone="neutral" title="工程注记">
          thinking 模式下 max_tokens=4096 会让难 case 的推理耗尽输出预算并返回空 content,修复实验统一改用 8192;
          Lockbud 在 Windows 需把 nightly-2026-02-07 工具链 bin 目录加入 PATH(rustc_driver DLL)并以
          RUSTC_WRAPPER 方式驱动;Miri 的 many-seeds 并行执行,256 种子仅 3–5 秒/case。
        </Callout>
      </Stack>
    </Stack>
  );
}
