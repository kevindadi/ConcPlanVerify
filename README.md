# cir2cvn

Translator from **CIR** (Concurrency Intermediate Representation) to **CVN** (Concurrency Verification Net).

## Overview

This crate implements a faithful 1:1 translation from CIR programs into CVN Petri nets
suitable for state-space exploration and deadlock/livelock detection. It is the bridge
between the CIR front-end (which extracts concurrency structure from source code) and
the CVN analysis back-end (which performs model checking).

```
Requirment ──(front-end)──▶ CIR ──(cir2cvn)──▶ CVN ──(analysis)──▶ Counterexample
```

## Usage

```rust
use cir2cvn::translate;

let program: cir::ast::Program = serde_json::from_str(&json)?;
let net = translate(&program)?;

// Use the CVN for analysis
let state = net.initial_state();
let enabled = net.enabled_transitions(&state);
```

## Project Structure

```
src/
├── lib.rs                   # Public API
├── error.rs                 # TranslateError (T0xx–T3xx)
├── validate.rs              # Post-translation checks
└── translator/
    ├── mod.rs               # Three-phase orchestration
    ├── context.rs           # Translation context
    ├── expr_parser.rs       # CIR string → CVN expression
    ├── resource.rs          # Phase 1: resource scanning
    ├── control_flow.rs      # Transfer planning
    ├── operation.rs         # Phase 2: operation translation
    ├── condvar.rs           # Condvar specialization
    └── fn_summary.rs        # FnSummary indexing
```

## Building

```bash
cargo build
cargo test
```

With LLM helpers (NL→CIR generation, repair loop):

```bash
cargo build --features llm
cargo test -p cir2cvn --features llm
```

## Desktop app (`cpn-gui`)

Tauri 2 + React 界面：自然语言需求 → LLM 生成 CIR、JSON 编辑与校验、CVN 翻译与状态空间分析（含 DOT 预览）、可配置 `uni-llm.toml` 路径与 provider/model 覆盖、LLM 修复循环。

### 准备

1. 安装 [Node.js](https://nodejs.org/) 与 [Rust](https://rustup.rs/)。
2. 按 [`uni-llm/README.md`](uni-llm/README.md) 准备 `uni-llm.toml`（API key 走环境变量，勿写入前端）。
3. 在应用 **设置** 页填写该文件的绝对路径或 `~/...`；默认值为仓库根下文件名 `uni-llm.toml`（从 `cpn-gui` 目录启动 dev 时需自行改为可解析路径）。

### 开发与打包

```bash
cd cpn-gui
npm install
npm run dev          # 仅 Vite；在浏览器打开时无 Tauri IPC，界面会用内置默认设置
```

不要只用浏览器访问 `http://localhost:1420` 当作完整应用；请用下面的 `cargo tauri dev` 启动带 Rust 后端的窗口。

```bash
cargo install tauri-cli --locked   # 若尚未安装
```

**Tauri CLI 2.x 的 `dev` / `build` 没有 `--manifest-path` 参数**；请用 **`-c`（`--config`）** 指向 `tauri.conf.json`，或在 `cpn-gui/` 下直接运行（CLI 会向上找到配置）。

| 你所在目录 | 开发命令 | 打包命令 |
|------------|----------|----------|
| 仓库根目录（`cpn-guide-llm/`） | `cargo tauri dev -c cpn-gui/src-tauri/tauri.conf.json` | `cargo tauri build -c cpn-gui/src-tauri/tauri.conf.json` |
| `cpn-gui/` 子目录 | `cargo tauri dev` | `cargo tauri build` |

在子目录下**不要**再写 `cargo tauri dev --manifest-path ...`：当前 CLI 会报 `unexpected argument`；也不要用 `cargo tauri dev -- --manifest-path ...`（`--` 后面是给应用 / runner 的参数，不是给 Tauri 的）。

### 手动 E2E 清单

1. **设置**：选择 `uni-llm.toml`，保存；可选填写 provider/model 覆盖。
2. **工作台**：输入简短并发需求 →「用 LLM 生成 CIR」→ 得到 JSON；点「校验 CIR」查看报告；必要时「LLM 修复」。
3. **CVN 分析**：切换 BFS/DFS 与 `max_states` →「翻译并分析」→ 查看状态数、死锁列表、DOT 预览或下载 `.dot`。
4. **导出**：工作台「导出 cir.json」。

说明：当前 vendored `cvn` 未包含业务目标可达性检查模块，`repair` 循环在存在 `goals` 时仅打印警告，不以目标不可达触发修复（死锁等 CVN 反例仍正常驱动修复）。

## Documentation

See the [`doc/`](doc/) directory:
- [Architecture](doc/architecture.md)
- [Translation Rules](doc/translation_rules.md)
- [Examples](doc/examples.md)
- [Error Codes](doc/error_codes.md)

## Dependencies

- **cir** (`ceir`) — CIR library (vendored in-repo under `cir/`)
- **cvn** — CVN library with `cir-anchor` feature (vendored in-repo under `cvn/`)
- **uni-llm** — optional LLM client for the `llm` feature (vendored in-repo under `uni-llm/`)
