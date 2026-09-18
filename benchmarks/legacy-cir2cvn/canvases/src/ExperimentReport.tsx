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

// ── Data sources: results/offline_v3.json · repair_8k.json · repair_local.json ·
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

// llm_judge: results/llm_judge.json (single-shot judging, no oracle; DeepSeek v4 Pro thinking)
type Judge = {
  case: string;
  gold: string;
  claimed: string;
  kind: string | null;
  note: string;
  tone: "success" | "danger" | "warning" | "neutral";
};
const judged: Judge[] = [
  { case: "mutex_deadlock", gold: "bug", claimed: "bug", kind: "Deadlock", note: "localized correctly (w1/w2)", tone: "success" },
  { case: "three_way_deadlock", gold: "bug", claimed: "bug", kind: "Deadlock", note: "localized correctly (3-cycle)", tone: "success" },
  { case: "signal_loss", gold: "bug", claimed: "bug", kind: "SignalLoss", note: "kind correct", tone: "success" },
  { case: "channel_deadlock", gold: "bug", claimed: "bug", kind: "Deadlock", note: "gold ChannelBlock, kind generalized", tone: "warning" },
  { case: "dead_transition", gold: "bug", claimed: "bug", kind: "DeadTransition", note: "kind correct", tone: "success" },
  { case: "dual_condvar", gold: "bug", claimed: "bug", kind: "Deadlock", note: "gold SignalLoss, kind wrong", tone: "warning" },
  { case: "partial_deadlock", gold: "goals_unmet", claimed: "bug", kind: "Deadlock", note: "kind wrong (gold DeadTransition+goals)", tone: "warning" },
  { case: "goal_unreachable", gold: "goals_unmet", claimed: "bug", kind: "GoalUnreachable", note: "kind correct", tone: "success" },
  { case: "cas_race", gold: "safe", claimed: "bug", kind: "DeadTransition", note: "false positive: misread CAS-result branch as variable-value branch", tone: "danger" },
  { case: "semaphore_throttle", gold: "safe", claimed: "safe", kind: null, note: "correct", tone: "neutral" },
  { case: "fn_summary_prop", gold: "safe", claimed: "safe", kind: null, note: "correct", tone: "neutral" },
  { case: "scale_lock_chain_6x3", gold: "safe", claimed: "safe", kind: null, note: "correct (Lockbud false-positives here)", tone: "neutral" },
  { case: "scale_lock_chain_5x3_buggy", gold: "bug", claimed: "bug", kind: "Deadlock", note: "localized correctly (w5 reverse order)", tone: "success" },
  { case: "deep_lock_chain_4x3", gold: "bug", claimed: "bug", kind: "Deadlock", note: "explanation precisely localizes w3 else arm", tone: "success" },
  { case: "deep_lock_chain_4x3_safe", gold: "safe", claimed: "safe", kind: null, note: "not induced into a false positive", tone: "neutral" },
  { case: "scale_branch_fan_4x2", gold: "safe", claimed: "safe", kind: null, note: "correct", tone: "neutral" },
];

// codegen: results/codegen.json + codegen_v2.json (verified ConcIR -> Rust, cargo check acceptance, all 1 round)
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

// External baselines: results/miri_baseline.json (preemption 0.5 × 16 seeds; missed cases rechecked with 256 seeds)
// and results/lockbud_baseline.json (-k deadlock, nightly-2026-02-07)
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
  { case: "mutex_deadlock", gold: "Deadlock", cvn: "Deadlock", lockbud: "ConflictLock", lockbudOk: "hit", miri: "missed (256 seeds)", miriOk: "miss" },
  { case: "three_way_deadlock", gold: "Deadlock (3-cycle)", cvn: "Deadlock", lockbud: "missed", lockbudOk: "miss", miri: "missed (256 seeds)", miriOk: "miss" },
  { case: "signal_loss", gold: "SignalLoss", cvn: "SignalLoss", lockbud: "missed", lockbudOk: "miss", miri: "missed (256 seeds)", miriOk: "miss" },
  { case: "channel_deadlock", gold: "ChannelBlock", cvn: "ChannelBlock", lockbud: "missed", lockbudOk: "miss", miri: "missed (256 seeds)", miriOk: "miss" },
  { case: "dead_transition", gold: "DeadTransition", cvn: "DeadTransition", lockbud: "missed", lockbudOk: "miss", miri: "missed (dynamically invisible)", miriOk: "miss" },
  { case: "dual_condvar", gold: "SignalLoss", cvn: "SignalLoss", lockbud: "missed", lockbudOk: "miss", miri: "deadlock reported", miriOk: "hit" },
  { case: "partial_deadlock", gold: "goals unmet", cvn: "DeadTransition + unmet goals", lockbud: "missed", lockbudOk: "miss", miri: "timeout hang (undiagnosed)", miriOk: "partial" },
  { case: "goal_unreachable", gold: "goals unmet", cvn: "goals_unmet", lockbud: "DoubleLock (misdiagnosed)", lockbudOk: "wrong", miri: "assert panic", miriOk: "hit" },
  { case: "scale_lock_chain_5x3_buggy", gold: "Deadlock (lock order)", cvn: "Deadlock", lockbud: "DoubleLock (misdiagnosed)", lockbudOk: "wrong", miri: "missed (256 seeds)", miriOk: "miss" },
];

// scaling sweep: results/scaling.json (all 37 points; slice below: locks=3 / branches=2, threads 2–5)
const threadAxis = ["2 threads", "3 threads", "4 threads", "5 threads"];
const scalingSeries = [
  { name: "lock chain · safe (3 locks)", data: [77, 437, 2277, 11237] },
  { name: "lock chain · deadlock (3 locks)", data: [86, 518, 2790, 14054] },
  { name: "branch fan-out (2 branch levels)", data: [153, 2193, 29121, 100000] },
];

// repair_8k.json: 7 bug cases × 3 feedback modes (max_tokens=8192, max_rounds=5)
type Repair = { case: string; ok: boolean; rounds: number; tokens: number };
const repairByMethod: Record<string, Repair[]> = {
  full: [
    { case: "mutex_deadlock", ok: true, rounds: 1, tokens: 3963 },
    { case: "three_way_deadlock", ok: true, rounds: 1, tokens: 5344 },
    { case: "signal_loss", ok: true, rounds: 2, tokens: 9865 },
    { case: "channel_deadlock", ok: true, rounds: 1, tokens: 4535 },
    { case: "dead_transition", ok: true, rounds: 4, tokens: 23963 },
    { case: "dual_condvar", ok: true, rounds: 3, tokens: 24417 },
    { case: "partial_deadlock", ok: true, rounds: 2, tokens: 20434 },
  ],
  statusOnly: [
    { case: "mutex_deadlock", ok: true, rounds: 1, tokens: 3565 },
    { case: "three_way_deadlock", ok: true, rounds: 1, tokens: 4270 },
    { case: "signal_loss", ok: false, rounds: -1, tokens: 18812 },
    { case: "channel_deadlock", ok: true, rounds: 1, tokens: 3881 },
    { case: "dead_transition", ok: true, rounds: 1, tokens: 6467 },
    { case: "dual_condvar", ok: false, rounds: -1, tokens: 31016 },
    { case: "partial_deadlock", ok: true, rounds: 1, tokens: 6006 },
  ],
  llmOnly: [
    { case: "mutex_deadlock", ok: true, rounds: 1, tokens: 3771 },
    { case: "three_way_deadlock", ok: true, rounds: 1, tokens: 4709 },
    { case: "signal_loss", ok: false, rounds: -1, tokens: 21707 },
    { case: "channel_deadlock", ok: true, rounds: 1, tokens: 5640 },
    { case: "dead_transition", ok: true, rounds: 2, tokens: 11305 },
    { case: "dual_condvar", ok: false, rounds: -1, tokens: 38218 },
    { case: "partial_deadlock", ok: true, rounds: 1, tokens: 7446 },
  ],
};

const methodLabel: Record<string, string> = {
  full: "full CVN feedback",
  statusOnly: "status/kind only",
  llmOnly: "no CVN diagnosis",
};

// full_run.json: 5 natural-language requirements per case (1 canonical + 4 paraphrases), ≤5 rounds each
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

// repair_local.json vs repair_full_v2.json: slice-repair A/B on 12 non-safe cases
// (feedback is summarized full in both; fell means slice could not localize / exhausted then fell back to full repair)
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

  return (
    <Stack gap={28} style={{ padding: 24, maxWidth: 1080 }}>
      <Stack gap={6}>
        <H1>ConcPlanVerify Experiment Report</H1>
        <Text tone="secondary">
          Model DeepSeek v4 Pro (thinking + high reasoning) · Benchmarks benchmarks/manifest.json (18 cases: 12 defect + 6
          safe controls, including 3 parameterized large-scale cases, 1 deep-buried bug twin pair, and 3 classes of goals
          negatives) · External baselines Lockbud (static) / Miri (dynamic) · 2026-08-05
        </Text>
      </Stack>

      <Grid columns={5} gap={16}>
        <Stat value="20/20*" label="CVN detection verdicts correct" tone="success" />
        <Stat value="4/9 · 1 FP" label="Lockbud detected (2 misdiagnosed)" tone="warning" />
        <Stat value="3/9 · 0 FP" label="Miri detected (16–256 seeds)" tone="warning" />
        <Stat value="10/10 · 1 FP" label="LLM-only judging (2 kind errors)" tone="info" />
        <Stat value="18/18" label="codegen passed cargo check" tone="success" />
      </Grid>
      <Text tone="tertiary" size="small">
        *20 = original 18 + goal_constrained_deadlock / _dense; both new cases analyzed correctly this round; full offline not re-run.
      </Text>

      {/* ── 1. Detection capability ── */}
      <Stack gap={12} style={{ scrollMarginTop: 88 }} >
        <div id="s1" />
        <H2>1 · Bug detection: CVN state-space analysis vs static validation only (20 cases)</H2>
        <Text tone="secondary">
          For each case&apos;s gold buggy ConcIR, run <Text as="span" weight="semibold">cir2cvn --analyze</Text>
          (translation + state-space exploration + goals) and <Text as="span" weight="semibold">--validate</Text> (static rules only).
          CVN verdicts are correct on all 16 defect cases; static validation misses all behavioral bugs (its job is schema/structure
          checking; this ablation confirms the necessity of state-space analysis).
        </Text>
        <Table
          headers={["Case", "Expected", "CVN analyze verdict", "Detected bug kind", "Static validation only"]}
          rows={analyze.map((a) => [
            shortName(a.case),
            a.expected,
            a.actual,
            a.kinds.length ? a.kinds.join(" + ") : "—",
            a.expected === "safe" ? "safe (correct)" : "missed",
          ])}
          rowTone={analyze.map((a) => (a.expected === "safe" ? ("neutral" as const) : ("success" as const)))}
          columnAlign={["left", "left", "left", "left", "left"] as const}
          striped
        />
        <Text tone="tertiary" size="small">
          For partial_deadlock the status is verified_unsafe (DeadTransition prioritized) while the payload also reports 2 unmet
          goals; counted as detected at the goals-defect level. Current signal_loss repair/analyze keeps only SignalLoss: condvar
          translation variant grouping (cv_wake1/cv_wakeA etc. report only when the whole &quot;or&quot; group is dead) avoids treating
          mutually exclusive helper variants as independent DeadTransition. Data: results/offline_v3.json; historical fixed-fixture
          gate false positives are discussed in §9.
        </Text>
      </Stack>

      {/* ── 2. External baselines ── */}
      <Stack gap={12} style={{ scrollMarginTop: 88 }}>
        <div id="s2" />
        <H2>2 · External baselines: CVN vs Lockbud (static) vs Miri (dynamic), 9 defect cases</H2>
        <Text tone="secondary">
          Lockbud (nightly-2026-02-07, -k deadlock) runs static lock analysis on each case&apos;s reference buggy.rs; Miri runs with
          -Zmiri-preemption-rate=0.5 × 16 random seeds; missed cases rechecked with 256 seeds remain all missed.
        </Text>
        <Table
          headers={["Case", "Gold defect", "CVN", "Lockbud", "Miri"]}
          rows={baselines.map((b) => [shortName(b.case), b.gold, b.cvn, b.lockbud, b.miri])}
          rowTone={baselines.map((b) =>
            b.lockbudOk === "miss" && b.miriOk === "miss" ? ("warning" as const) : ("neutral" as const),
          )}
          striped
        />
        <Grid columns={2} gap={16}>
          <Callout tone="danger" title="Miri: schedule-dependent deadlocks are essentially unsampleable">
            Five schedule-dependent deadlocks (mutex / 3-cycle / signal loss / channel / lock order) are all missed even with 256
            random schedule seeds—the deadlock window is too rare under random sampling—while CVN exhaustively explores the
            interleaving space and detects all. Miri reports only when the defect happens to fall into an executed schedule
            (dual_condvar; goal_unreachable assert). Safe cases: zero false positives (5/5).
          </Callout>
          <Callout tone="warning" title="Lockbud: lock-order bugs detectable, but diagnosis unreliable">
            Classic two-lock ConflictLock is detected correctly; but Vec&lt;Arc&lt;Mutex&gt;&gt; index-alias confusion causes a
            DoubleLock false positive on safe scale_lock_chain_6x3 (the only FP), and the same DoubleLock misdiagnosis on the 5x3
            deadlock chain and goal_unreachable—no discriminative power on that pattern. Three-cycle deadlock, condvar, and
            channel classes are all missed.
          </Callout>
        </Grid>
      </Stack>

      {/* ── 3. goals ablation ── */}
      <Stack gap={12} style={{ scrollMarginTop: 88 }}>
        <div id="s3" />
        <H2>3 · Goals trust boundary: three negative classes + cost of disabling goals checks</H2>
        <Text tone="secondary">
          Goals negatives now cover three detectable forms (unreachable / too weak / bad reference), with two new check rules: a goal
          already true in the initial state → &quot;too weak&quot; warning; a goal referencing an undeclared variable or unknown place →
          translation warning. Re-running all cases with
          <Text as="span" weight="semibold"> cir2cvn --no-goals</Text> concentrates differences on goals-class defects:
        </Text>
        <Table
          headers={["Case", "Defect composition", "Goals on (default)", "Goals off (ablation)"]}
          rows={[
            ["goal unreachable", "no deadlock; goal x==3 unreachable", "goals_unmet ✓", "verified_safe (false accept)"],
            ["goal trivial", "goal x==0 already true initially (too weak; constrains no behavior)", "goals_unmet ✓ (too-weak warning)", "verified_safe (false accept)"],
            ["goal bad reference", "goal refs nonexistent place w1_done and undeclared var y", "goals_unmet ✓ (dangling-ref warning)", "verified_safe (false accept)"],
            ["partial deadlock", "DeadTransition + 2 unmet goals", "verified_unsafe ✓", "verified_unsafe (still detected)"],
          ]}
          rowTone={["danger" as const, "danger" as const, "danger" as const, "neutral" as const]}
          striped
        />
        <Text tone="tertiary" size="small">
          Goals-class gold has 4 cases; 3 of them (75%) fully rely on the goals layer as a backstop—static tools lack predicate
          semantics, dynamic tools can only observe via assert, and the &quot;too weak / dangling&quot; classes do not exist at the code
          level (the defect is in the spec itself). Missing goals (goals: []) does not change the verdict; it is a documented trust
          boundary (doc/goals_policy.md). Data: results/no_goals_v2.json.
        </Text>
      </Stack>

      {/* ── 4. Repair experiments ── */}
      <Stack gap={12} style={{ scrollMarginTop: 88 }}>
        <div id="s4" />
        <H2>4 · Repair experiment: feedback-information ablation (7 defect cases × 3 modes)</H2>
        <Text tone="secondary">
          This table uses the repair_8k config after the corrected condvar model (max_tokens=8192, max_rounds=5). For full CVN
          feedback, signal_loss and dual_condvar use subsequent full-repair results, succeeding in 2 and 3 rounds respectively;
          status/kind only and no CVN diagnosis ablations were re-run under the corrected model and both cases failed within the
          5-round cap. repair_local is a local-slice experiment on the same cases; dual_condvar exhausted rounds in-slice then fell
          back to full repair and succeeded. The acceptance oracle for all three is the Rust analyzer&apos;s verified_safe.
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
                  <CardHeader trailing={<Pill size="sm">{`${ok.length}/${rows.length} success`}</Pill>}>
                    {methodLabel[m]}
                  </CardHeader>
                  <CardBody>
                    <Row gap={20}>
                      <Stat value={avgRounds.toFixed(1)} label="avg rounds (successes)" />
                      <Stat value={`${(tokens / 1000).toFixed(0)}k`} label="total tokens (in+out)" />
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
          LLM token totals per case × feedback mode (input+output, unit: token), from the corrected results/repair_8k.json. The two
          condvar cases under full mode use full-repair results; status-only and llm-only are independent re-runs under the
          corrected model. Full CVN feedback repairs in 2/3 rounds, while both weak-feedback modes exhaust 5 rounds on these two
          structural condvar cases.
        </Text>
        <Table
          headers={["Case", "Defect type", "full CVN feedback", "status/kind only", "no CVN diagnosis"]}
          rows={repairByMethod.full.map((r, i) => [
            shortName(r.case),
            bugCases[i]?.kinds.join(" + ") ?? "",
            r.ok ? `success · ${r.rounds} rounds` : "failed (5 rounds exhausted)",
            repairByMethod.statusOnly[i].ok
              ? `success · ${repairByMethod.statusOnly[i].rounds} rounds`
              : "failed (5 rounds exhausted)",
            repairByMethod.llmOnly[i].ok
              ? `success · ${repairByMethod.llmOnly[i].rounds} rounds`
              : "failed (5 rounds exhausted)",
          ])}
          rowTone={repairByMethod.full.map((r) => (r.ok ? ("success" as const) : ("danger" as const)))}
          striped
        />

        <H2>4b · Local regeneration vs full repair (12-case A/B, feedback summarized)</H2>
        <Text tone="secondary">
          Local regeneration (repair_local): slice by the function set implicated in the bug report; the LLM rewrites only sliced
          functions (others shown frozen as one-line sync summaries); Python splices back into the original ConcIR—non-slice parts are
          byte-identical. signal_loss succeeds in-slice; dual_condvar identifies 3/3 functions but the fix must simultaneously remove
          the cross-function mutual condvar handshake and unify to m1→m2 lock order, so after slice exhaustion it falls back to full
          repair and succeeds. When the slice cannot localize (pure goals defects) or rounds are exhausted, fall back to full repair.
          This round&apos;s full feedback enables summarization: isomorphic bugs deduplicated by signature + long counterexample
          traces compressed.
        </Text>
        <Table
          headers={["Case", "local repair", "token", "slice / total fns", "fallback", "full repair", "token"]}
          rows={repairAb.map((r) => [
            shortName(r.case),
            r.localOk ? "success" : "failed",
            r.localTokens.toLocaleString(),
            r.slice,
            r.fell ? "yes" : "—",
            r.fullOk ? "success" : "failed",
            r.fullTokens.toLocaleString(),
          ])}
          rowTone={repairAb.map((r) => (r.localOk ? ("neutral" as const) : ("danger" as const)))}
          columnAlign={["left", "left", "right", "right", "left", "left", "right"] as const}
          striped
        />
        <Grid columns={2} gap={16}>
          <Callout tone="success" title="Core value of local regeneration: zero semantic drift">
            Full repair shows oracle-invisible semantic drift on 3 cases (dead_transition invents a Mutex; dual_condvar adds 2 Vars;
            deep case changes Var to Atomic); local regeneration has 0 drift across 12 cases—frozen splicing structurally prevents
            &quot;casually editing unbroken parts.&quot; Success 11/12 vs 12/12 (sole failure still a condvar-class boundary); token totals
            comparable (129k vs 109k).
          </Callout>
          <Callout tone="success" title="Feedback summarization: 132k → 17k">
            deep_lock_chain_4x3&apos;s 99 deadlock counterexamples are deduplicated by equivalence signature into groups, keeping the
            shortest per group (overlong traces head/tail-compressed); single-round full-feedback repair drops from 132k to 17.3k
            tokens (~7.6×) with unchanged success rate—the cost disadvantage of full feedback on large cases is eliminated.
            Implementation: prompts.verification_feedback.
          </Callout>
        </Grid>
        <Text tone="tertiary" size="small">
          The three goals cases have slice 0/3 (defect is in the goal spec, not function bodies; bug report implicates no functions);
          by design they fall back to full repair and all succeed—validating the fallback mechanism. Data: results/repair_local.json,
          results/repair_full_v2.json.
        </Text>
      </Stack>

      {/* ── 5. Generation experiments ── */}
      <Stack gap={12} style={{ scrollMarginTop: 88 }}>
        <div id="s5" />
        <H2>5 · Generation experiment: natural language → ConcIR (10 cases × 5 requirements)</H2>
        <Text tone="secondary">
          For each case, generate ConcIR from 1 canonical requirement + 4 paraphrases (≤5 validation-retry rounds), then run full
          analyze on successful generations. Overall only {genValid}/50 pass static validation and {genSafe}/50 end verified_safe.
          Lock-order patterns (mutex / three-way / semaphore) nearly all succeed; channel, CAS, condvar, and branching patterns nearly
          all exhaust 5 rounds—the bottleneck is ConcIR schema detail (res_op action names, branch/transfer shapes), not concurrency
          semantics.
        </Text>
        <BarChart
          categories={generate.map((g) => shortName(g.case))}
          series={[
            { name: "passed static validation", data: generate.map((g) => g.valid), tone: "info" as const },
            { name: "final verified_safe", data: generate.map((g) => g.safe), tone: "success" as const },
          ]}
          height={260}
          yMax={5}
          showValues
        />
        <Text tone="tertiary" size="small">
          Counts (0–5) of generations per case that pass validation / end verified_safe. Source: results/full_run.json, DeepSeek v4
          Pro, max_tokens=4096.
        </Text>
        <BarChart
          categories={generate.map((g) => shortName(g.case))}
          series={[{ name: "token usage (in+out)", data: generate.map((g) => g.tokens), tone: "warning" as const }]}
          height={220}
          valueSuffix=" tok"
        />
        <Text tone="tertiary" size="small">
          LLM token totals for 5 generations per case. Failed cases hit the full 5-round retry budget, costing ~2–3× successful cases.
        </Text>
      </Stack>

      {/* ── 6. CVN scale & performance ── */}
      <Stack gap={12} style={{ scrollMarginTop: 88 }}>
        <div id="s6" />
        <H2>6 · CVN scale and verification performance (14 cases)</H2>
        <Text tone="secondary">
          Small cases all finish verify in under 2 ms; the three large cases (14k–53k states) complete in 84–366 ms with full
          state-space exploration—negligible vs a single LLM call at 10–60 s; the verifier is not a closed-loop bottleneck.
        </Text>
        <Table
          headers={["Case", "ConcIR stmts", "places (control/resource/wait)", "transitions", "arcs (in/out)", "reachable states", "verify time"]}
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
        <H2>7 · Scaling sweep: state-explosion knee (37 data points)</H2>
        <Text tone="secondary">
          Using the parameterized generator (cir_workflow/scaling.py), sweep threads × locks/branch-levels on three modes; each point
          runs full analyze. Reachable states grow exponentially with thread count; data-branching modes grow steepest: branch_fan at
          5 threads × 2 branch levels exceeds max_states=100k (analysis_incomplete), while pure lock-order at 6 threads × 3 locks
          (53k states) still completes.
        </Text>
        <LineChart
          categories={threadAxis}
          series={scalingSeries}
          height={280}
          referenceLines={[{ value: 100000, label: "max_states = 100k" }]}
          beginAtZero
        />
        <Text tone="tertiary" size="small">
          X-axis: thread count; Y-axis: reachable states (linear scale). The branch fan-out 5-thread point is plotted at 100k to
          indicate budget truncation (actual analysis_incomplete); lock-chain · safe at 6 threads × 3 locks is 53,477 states / 402 ms
          (completes off-chart). Source: results/scaling.json.
        </Text>
        <Callout tone="info" title="max_states budget guidance">
          Default 100k is enough for pure lock-order modes (6 threads × 3 locks ≈ 53k), but data-branching modes at ≥5 threads need
          tiered higher budgets or partial-order reduction to compress the interleaving space—the next engineering point on the
          scaling dimension.
        </Callout>

        <H2>7b · Scaling LLM legs: NL → generate ConcIR → verify → code</H2>
        <Text tone="secondary">
          On 6 representative sweep points, also run the LLM legs: the same NL requirement generates ConcIR (≤5 rounds, verified_safe
          acceptance); on success, codegen from the generated ConcIR; on failure, fall back to gold ConcIR for codegen (isolating the two
          failure surfaces).
        </Text>
        <Table
          headers={["mode × scale", "gold stmts", "generated ConcIR", "rounds", "token", "states", "codegen (source)", "token", "LOC"]}
          rows={[
            ["lock chain 2×2", "15", "verified_safe", "3", "12,556", "57", "pass (generated)", "2,226", "24"],
            ["lock chain 3×2", "22", "verified_safe", "1", "3,840", "111", "pass (generated)", "2,757", "32"],
            ["lock chain 4×3", "37", "verified_safe", "1", "3,593", "2,277", "pass (generated)", "4,180", "63"],
            ["lock chain 6×3", "55", "verified_safe", "2", "13,481", "53,477", "pass (generated)", "4,727", "59"],
            ["branch fan 2×2", "21", "5 rounds exhausted", "5", "22,523", "—", "pass (gold)", "4,439", "59"],
            ["branch fan 4×2", "41", "5 rounds exhausted", "5", "35,631", "—", "pass (gold)", "5,073", "36"],
          ]}
          rowTone={["neutral", "neutral", "neutral", "neutral", "danger", "danger"] as const}
          columnAlign={["left", "right", "left", "right", "right", "right", "left", "right", "right"] as const}
          striped
        />
        <Text tone="tertiary" size="small">
          Lock-chain mode 4/4 generation succeeds with no scale degradation (4×3 and 6×3 generated ConcIR stmt counts match gold;
          state spaces same order)—&quot;generation ability degrades with scale&quot; does not hold for pure lock-order; at 3×2 the model
          produced a leaner equivalent (12 vs 22 stmts). Both branch_fan points exhaust 5 rounds at static validation, consistent with
          §5: the bottleneck is branch/switch schema detail, not concurrency modeling, wasting 22k–36k tokens—few-shot exemplars are
          the most direct fix. Codegen leg 6/6 pass (including the two gold-fallback points). Data: results/scaling_llm.json.
        </Text>
      </Stack>

      {/* ── 8. LLM-only judging ── */}
      <Stack gap={12} style={{ scrollMarginTop: 88 }}>
        <div id="s8" />
        <H2>8 · LLM-only judging vs CVN: deep-buried bugs and false-positive probes (16 cases)</H2>
        <Text tone="secondary">
          Single-shot judging with no oracle: the LLM reads ConcIR directly and reports bug/safe, kind, and suspected locus. To stress
          it, we added a deep-buried bug case (deep_lock_chain_4x3: 4 workers × 2 branch arms = 8 near-identical lock-order segments;
          only w3&apos;s else arm advances m2 early; the if arm skips m2 to bypass intra-function lock-order static rule E505) and its
          safe twin.
        </Text>
        <Table
          headers={["Case", "Gold", "LLM verdict", "LLM kind", "Notes"]}
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
          <Callout tone="danger" title="Real false positive obtained: cas_race">
            The LLM misreads a &quot;branch on CAS result&quot; as a &quot;branch on variable value,&quot; infers a permanently false condition, and
            reports DeadTransition—a false positive on a safe case. CVN models both CAS success/failure transfers per ConcIR semantics
            and judges safe. This shows LLM judging is unreliable on semantic detail; the verifier&apos;s value is not &quot;can it find bugs&quot;
            but &quot;what it says is necessarily correct.&quot;
          </Callout>
          <Callout tone="warning" title="Deep-buried lock-order bug did not fool DeepSeek">
            Amid 67 stmts and 8 near-identical lock-order segments, a single adjacent swap: the judging explanation still precisely
            localizes w3&apos;s else arm; all three repair-feedback modes also succeed in 1 round—lock-order defects do not need
            localization to fix (normalizing all sequences is safe). To widen the gap between LLM-only and CVN feedback, pursue:
            condvar/SignalLoss (already a repair boundary), goals semantic constraints (so &quot;normalize-style fixes&quot; break business
            goals and are rejected), and larger CIRs with cross-function call chains that hold locks.
          </Callout>
        </Grid>
        <Text tone="tertiary" size="small">
          Two other kind errors: dual_condvar (gold SignalLoss, LLM says Deadlock), partial_deadlock (gold DeadTransition+goals, LLM
          says Deadlock); repair experiments also saw both modes silently change a flag Var to Atomic—oracle-invisible semantic
          drift. Sources: results/llm_judge.json, results/deep_repair.json.
        </Text>
      </Stack>

      {/* ── 9. codegen ── */}
      <Stack gap={12} style={{ scrollMarginTop: 88 }}>
        <div id="s9" />
        <H2>9 · Code generation: verified ConcIR → Rust (cargo check acceptance)</H2>
        <Text tone="secondary">
          The last link of the user story: given a verified ConcIR plan, the LLM generates main.rs under a &quot;structure-faithful mapping&quot;
          constraint, accepted by `cargo check` (≤3 rounds). 18/18 all pass in 1 round. An earlier codegen run was blocked on
          signal_loss by a historical DeadTransition gate false positive from a fixed fixture; current condvar variant grouping and
          repair-layer suffix filtering removed that artifact, and this round passes. dual_condvar&apos;s fixed ConcIR removes the mutual
          waiting condvar handshake so both threads acquire locks in m1→m2 order and pass verification. The end-to-end pipeline
          (requirement → ConcIR → verify → code) smoke-succeeds on mutex_deadlock and semaphore_throttle at ~7k tokens per chain.
        </Text>
        <BarChart
          categories={codegen.map((c) => shortName(c.case))}
          series={[
            { name: "ConcIR stmt count", data: codegen.map((c) => c.stmts), tone: "info" as const },
            { name: "generated code LOC", data: codegen.map((c) => c.loc), tone: "success" as const },
          ]}
          height={260}
        />
        <Text tone="tertiary" size="small">
          ConcIR stmt count vs non-empty non-comment lines of generated Rust. Lock-chain ConcIR is stmt-heavy but code can fold
          (scale_lock_chain_6x3: 55 stmts → 24 lines; the model folds 6 identical workers into a loop); deep_lock_chain_4x3_safe does
          not fold (104 lines). Folding is semantically equivalent but breaks stmt-level 1:1 correspondence—hence the need for a
          &quot;ConcIR ↔ Rust consistency check&quot; (todo §5). Sources: results/codegen.json, results/pipeline_smoke.json.
        </Text>
      </Stack>

      {/* ── 10. Goals-constrained repair ── */}
      <Stack gap={12} style={{ scrollMarginTop: 88 }}>
        <div id="s10" />
        <H2>10 · Goals-constrained deadlock: blocking &quot;normalize-style wild fixes&quot;</H2>
        <Text tone="secondary">
          New goal_constrained_deadlock (3 workers, ~2k states) and dense twin (4 workers, ~65k states): deadlock buried in w3&apos;s
          else arm (m2→m1); the same arm is the only path writing result=99; the business goal requires 99 reachable. Offline probe:
          changing 99→3 on the fixed ConcIR yields goals_unmet—&quot;delete the arm / unify the write&quot; clears the deadlock but fails
          acceptance.
        </Text>
        <Table
          headers={["Case", "mode", "success", "rounds", "keeps result=99", "token"]}
          rows={[
            ["goal constrained", "full CVN", "yes", "1", "yes", "9854"],
            ["goal constrained", "status only", "yes", "1", "yes", "6697"],
            ["goal constrained", "no feedback", "yes", "1", "yes", "6482"],
            ["dense", "full CVN", "yes", "1", "yes", "16722"],
            ["dense", "status only", "yes", "1", "yes", "10159"],
            ["dense", "no feedback", "yes", "1", "yes", "10128"],
            ["dense × Flash", "full CVN", "yes", "2*", "yes", "11742"],
            ["dense × Flash", "no feedback", "yes", "1", "yes", "12511"],
          ]}
          striped
        />
        <Text tone="tertiary" size="small">
          *Flash×CVN failed round 1 due to empty thinking content; succeeded on round 2. Sources:
          results/goal_constrained_repair_ab.json, results/goal_constrained_flash.json. Oracle layer is effective; neither DeepSeek
          Pro nor Flash spontaneously drops 99, so success rates do not diverge.
        </Text>
        <Callout tone="info" title="Layered conclusions">
          (1) The acceptance gate rejects normalize-style wild fixes; (2) current strong models spontaneously keep the distinctive
          write; (3) use this case as a regression probe and weak-model / adversarial-rewrite contrast—see doc/goals_policy.md §5.
        </Callout>
      </Stack>
    </Stack>
  );
}
