# experiments-v2（A 段迁移 + B 段离线 harness）独立复核

日期：2026-09-18。范围：ConcPlanVerify 提交 `4b45e7e` 工作树、ConcIR 未提交工作树、论文目录。未读取 `.env`，未发起任何模型请求，未修改两个代码仓库。

## 结论

A 段（仓库重组）接受：三仓布局符合约定，7043 个文件迁移前后 sha256 一致，论文目录只剩 tex/bib/sty/figures，ConcPlanVerify 工作树干净且已提交。

B 段接受为"离线 harness 骨架已实现并跑通"，不接受为"检测能力对比已完成"或"实验框架可直接进入 live"。原因：9 个模式只完成 P1/P4；Track D 的 miri 结果缺少原始输出无法复核；终审 oracle 的行为测试存在空跑即通过；ConcIR 当前工作树不能编译。以下问题需在下一轮先修。

## 复核证据

- Python 全部 72 项测试重跑 OK（`PYTHONPATH=python:. python3 -m unittest discover -s python/tests -t .`）。
- `git -C ConcPlanVerify status --short` 为空；`git ls-files | rg /target/` 为 0，`.git` 26 MB。
- `docs/MIGRATION_2026-09-18.md`：7043/7043 校验一致，无 >5 MB 文件，无 .env 迁移。
- 独立重跑 miri：在 `experiments/detection-v1/rust_projects/_rust_P1_buggy/probe` 下以 `MIRIFLAGS="-Zmiri-seed=<s> -Zmiri-preemption-rate=0.5"` 跑 seed 4/7/11/23/42，均 exit 0、无 deadlock 报告；`MIRIFLAGS="-Zmiri-bogus"` 得到 `error: unknown unstable option: miri-bogus`，证明 MIRIFLAGS 确实传入。因此"miri 5 个 seed 未检出 P1"的结论可信，但样本量偏小（见 P2-3）。
- ConcIR：`cargo build --release --offline --bin concir-backend` 失败（edition 2024 下 `validate/types.rs:397` 等 `ref` 绑定错误）。detection-v1 使用的二进制 sha `73b5dd64…` 是上一轮的旧 binary。

## 必须先修（P1）

### P1-1 工具原始输出未归档
`rust_arm.ToolRun.as_dict(include_raw=False)` 只写 sha256；`experiments/detection-v1/` 中没有任何 miri/cargo 的 stdout/stderr 文件，`rust_projects/*/probe` 只有源码与 target。审阅者无法确认 miri 输出内容、无法复核 `classify_detection` 的文本分类。B 段 prompt 明确要求"原始 stdout/stderr 全部存档"。修法：每次工具调用落盘 `calls/<id>/{argv.txt,env.txt,stdout.txt,stderr.txt,exit.txt}`，记录中保存路径与哈希；`env.txt` 至少含 `MIRIFLAGS`。

### P1-2 行为测试空跑即通过
P1 buggy 的 `behavior_test_ok: true`，但项目内没有任何 `#[test]`（handoff 自己也说行为测试尚未编写）。`cargo test --quiet` 在零测试时 exit 0 被记为通过。终审 oracle 因此对 buggy 程序给出"行为正常"。修法：解析 `test result: ok. N passed` 且要求 N ≥ 1，否则 `behavior_test_ok=None` 并记录 `no_tests`；同时为每个模式编写真实行为测试。

### P1-3 ConcIR 工作树不可编译且含杂项
未提交改动：`Cargo.toml` edition 2021→2024、`src/ast.rs` 一处 `ref` 修复、根目录多出文件名带前导空格的 `" rust-toolchain.toml"`（`channel = "nightly-2025-10-27"`）、迁移导致的 `scripts/` 与 `doc/todo.md` 删除及 `.gitignore` 变更。handoff 称 edition 变更为外部编辑；无论来源，当前状态下无法产出新 binary，后续任何 Rust 侧改动（B7 标签、修复缺陷）都会被阻塞。需要用户决定：回退到 2021，或补齐 `validate/control.rs`、`validate/types.rs` 的两处 `ref` 修复并保留 2024；删除带空格的文件名（如需 toolchain 固定，改为正确的 `rust-toolchain.toml` 并在文档中说明）。

## 应修（P2）

### P2-1 基准覆盖 2/9
P2、P3、P5、P6、P7、P8、P9 的模块化 CIR、contract、spec、行为测试均未编写，Track D 与 live 都只能覆盖 P1/P4。这是进入任何对比实验前的主要工作量。

### P2-2 P1 buggy CIR 的来源需声明
`benchmarks/patterns/P1/buggy.cir.json` sha256 `d2d4958b…` 与 `deepseek-flash-pilot-v1` 中 t2_abba 的 LLM 生成冻结模型完全相同。作为"人写 ground truth"使用时必须在 `ground_truth.json`/MANIFEST 中声明 `provenance: llm_generated (pilot-v1 t2_abba)`，或者由人重新编写并注明。

### P2-3 miri 样本量与错误分类
每个 seed 运行约 0.2 s，N=5 明显偏少，会让"miri 漏报"结论显得刻意。建议在协议中登记偏差：N 提升到 ≥ 64（本机 miri 支持 `-Zmiri-many-seeds=0..64` 时优先使用，否则循环 seed），保留原 5 组结果作为首批。另外 `classify_detection` 只看文本，miri 因非 bug 原因 exit≠0（如不支持的 API、`--offline` 依赖问题）会被记为 `detected=[]`，应单列 `tool_error`。

### P2-4 A3 臂的轮次记录为空
`ARMS_OFFLINE.json` 中 `A3_ours_revision` 的 `rounds: []` 而 `accepted_round: 1`。revision 工作流没有把每轮（请求哈希、工具耗时、决策）写入 arm 记录，live 时 tokens/时间将无法按轮汇总。

### P2-5 记录中缺少环境变量
`argv` 中看不到 `MIRIFLAGS`，仅凭记录无法知道该次运行的 seed/preemption 是否真的生效（本次是我用 bogus flag 侧面验证的）。与 P1-1 一并修。

## 未完成项（handoff 已如实列出）

- lockbud 未安装，臂状态 `unavailable`。
- A0/A1/A2 的 Rust 生成/审查/工具反馈 prompt 未写。
- `arms.py` live 入口 fail-closed，未接线。
- B7 细化标签未做（协议允许）。

## 对论文方向的提示

Track D 首批数据已经能说明一个有价值的点：ConcIR 在 P1/P4/rmw-zenoh-998 上完整 FAIL/PASS 分辨（49 状态、4 ms），而 miri 在 10 个不同 seed 上均未触发 ABBA 死锁。这个对比只有在原始输出可复核、样本量足够、P2/P3/P6 等非锁序模式也覆盖后才能写进论文。
