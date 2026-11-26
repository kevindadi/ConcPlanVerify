# ConcPlanVerify v4 Architecture

ConcPlanVerify is organized into clearly separated formal backends (`backends/`,
Rust) and a PEP 621 `src/`-layout Python orchestration package
(`src/concplan/`). All formal semantics—lowering, validation, Petri-net
exploration, compositional lock-order summaries, deterministic repair, codegen,
and stuttering refinement—live in Rust; Python owns LLM prompting, workflow
control loops, static verifier composition, and traceability reporting.

See [`docs/REFINEMENT_SPEC.md`](REFINEMENT_SPEC.md) for exact JSON schemas and
CLI exit codes.

---

## 1. Formal Backends (Rust — `backends/`)

### 1.1 `backends/concir/` (git submodule)

The `concir-backend` CLI implements the ConcIR v4 formal engine:
- **Structured Surface IR (`SCIR` v4.0, `concir-backend lower`):** Lowers
  structured control-flow constructs (`lock_guard`, `if_else`, `while_loop`,
  `while_wait`, `stmt`) into flat CIR statements with deterministic `s1, s2, ...`
  `sid`s and emits `concir-trace-map/1` preserving `structured_id`, `hole_id`,
  and `req` traceability tags.
- **First-class `seq_hole`:** Represents pure sequential computation with
  optional modeled state updates (`updates_modeled`) and expected Rust type
  (`expected_type`), executed as an atomic silent step (`tau`) in the
  interpreter and Petri-net engine.
- **Model Verification (`check`, `support`, `explore`, `summarize`):** Static
  validation, supportability check, exhaustive Petri-net / interpreter
  verification against frozen contracts, and compositional module summaries
  (`concir-module-summary/1`) with cross-module lock-cycle detection.
- **MIR-Clean `codegen`:** Emits a std-only Rust crate where runtime tracing is
  feature-gated (`cir_trace`), `Atomic` resources emit real `AtomicI64` /
  `AtomicBool`, lock-protected `Var` resources emit lock-free `SyncVar<T>`, and
  `codegen.json` maps resource FQNs and sequential holes (`/* HOLE(id) */`).
- **Stuttering Refinement (`verify-refinement`):** Checks Branch-Pruning
  Stuttering Refinement (`concir-refinement-report/1`) between the verified CIR
  specification and an exported implementation Petri net (`concir-impl-net/1`),
  verifying `O1` hole purity, `O2` weak simulation over sync resource
  primitives, deadlock-freedom, lock-divergence freedom, and preserved goals.

### 1.2 `backends/spn/` (`spn`)

A `rustc_driver`-based MIR analyzer invoked via `RUSTC_WRAPPER=spn` with
`--no-default-features` (eliminating runtime trace mutexes from MIR). Passing
`SPN_FLAGS="--export-concir-net <impl_net.json> --codegen-map <codegen_filled.json>"`
exports the compiler MIR synchronization Petri net (`concir-impl-net/1`),
canonicalizes resource names via `codegen_filled.json`, and attributes any
synchronization or loop summary inside a hole's `[start_line, end_line]` range
as a `purity_violation`.

### 1.3 `backends/third_party/` (Optional Local Analyzers)

Optional gitignored external Rust static analyzers (such as `lockbud` and
`rudra`). When `backends/third_party/lockbud/target/{release,debug}/lockbud` is
built (or `CONCIR_LOCKBUD` is set), `LockbudPlugin` can be composed into
`CompositeStaticVerifier`.

---

## 2. Python Orchestration Layer (`src/concplan/`)

### 2.1 `concplan.core`

Low-level infrastructure shared across all workflows:
- `toolchain.py` — Backend-first, Windows `.exe`-safe binary discovery
  (`find_concir_backend`, `find_spn`, `find_lockbud`, `find_concir_instrument`,
  `find_bind_check`) resolving `backends/concir/target/{release,debug}`,
  `backends/spn/target/{release,debug}`, and
  `backends/third_party/lockbud/target/{release,debug}`, plus `ToolchainConfig`.
- `concir_client.py` — Typed subprocess wrapper (`ConcirClient`,
  `ConcirResult`, `ConcirIdentity`) for `check`, `support`, `explore`, `lower`,
  `summarize`, `verify_refinement`, `repair`, `replay`, `repair_context`, and
  `evaluate_patch`. Distinguishes `semantic`, `usage_error`, `protocol_error`,
  and `process_error`.
- `models.py` — `ModelConfig`, `Consumption` token/wall-clock accounting, and
  `normalize_token_usage`.
- `llm.py` — OpenAI-SDK-backed `DeepSeekClient` (Chat Completions), `QwenClient`
  (Responses API), `LlmClient` protocol, and `create_llm_client` factory.
- `providers.py` — `CandidateProvider`, `ScriptedProvider`,
  `LlmCandidateProvider`, `PatchCandidateProvider`, and `ScriptedPatchProvider`.
- `prompts.py` — `importlib.resources`-backed loader and SHA-256 registry for
  the 4 bundled prompt assets in `concplan.prompts`, plus structured
  check/explore/patch feedback builders.
- `env.py` & `json_utils.py` — `.env` loader, defensive JSON extraction
  (`extract_json`), and SHA-256 helpers (`sha256_text`, `sha256_bytes`,
  `sha256_file`).

### 2.2 `concplan.model_verify`

Model-level verification, normalization, and repair loops:
- `normalize.py` — Deterministic, recorded Tier-1 CIR JSON normalization
  (field aliases, type/resource canonicalization, `sid` hygiene).
- `offline_workflow.py` — `OfflineWorkflow`: bounded generation + `check`
  retries, freezes the first valid CIR, then runs `support`, `explore`, and
  deterministic backend `repair` + `replay`.
- `revision_workflow.py` — `RevisionWorkflow` (`WholeArtifactRevisionWorkflow`):
  iterative whole-artifact or function-scoped (`local`) CIR revision driven by
  structured backend diagnostics against a frozen contract.
- `patch_repair.py` — `ExternalPatchRepairWorkflow`: constrained single-patch
  repair loop (`repair_context` -> propose -> `evaluate_patch` -> `replay`).

### 2.3 `concplan.code_verify`

Post-codegen static and dynamic Rust verification:
- `conformance.py` — `codegen`, `holes_of`, `fill_holes`, `lint_filled`,
  `collect_traces` (native & Miri), and `conform_all`.
- `static_verify.py` — Pluggable static verification architecture:
  - `StaticViolation`, `StaticVerificationReport`, `StaticVerifierPlugin`,
    `CompositeStaticVerifier`
  - `fill_holes_marked` & `augment_codegen_map_with_hole_ranges` (computes exact
    multi-line `[start_line, end_line]` spans in `codegen_filled.json`)
  - `AstHolePurityPlugin`: skeleton integrity (`// @cir` sid multiset +
    outside-hole diff) and comment/string-safe `O1` hole purity checks
  - `SpnMirRefinementPlugin`: runs `cargo check --offline --no-default-features`
    with `RUSTC_WRAPPER=spn` (auto-detecting `backends/spn/rust-toolchain.toml`
    and injecting toolchain runtime PATH/LD env) and invokes
    `ConcirClient.verify_refinement`
  - `LockbudPlugin`: self-contained optional static deadlock detector wrapping
    `backends/third_party/lockbud`
- `bounded_monitor.py` — Wraps `concir-backend monitor` over observed trace runs
  and computes bounded requirement coverage (`RC`/`RF`).

### 2.4 `concplan.pipeline`

- `e2e_pipeline.py` — `run_e2e_pipeline`: connects Stage 1/2 (Loop A
  SCIR/CIR generation, lowering, optional `summarize`, exhaustive `explore`),
  Stage 3 (`codegen` + marked hole filling), and Stage 4
  (`CompositeStaticVerifier` + Loop B offending-hole-only repair + Loop C
  cross-layer escalation + optional dynamic `conform`). Always emits
  `traceability_manifest.json` (`concir-traceability-manifest/1`).
- `benchmarks.py` — `BenchmarkTask`, `list_benchmarks`, `load_benchmark`, and
  `benchmarks_summary` for `benchmarks/families/` and `benchmarks/real-cases/`.

---

## 3. Bundled Prompts & Benchmarks

- **`src/concplan/prompts/`** (`concplan.prompts`, packaged via
  `[tool.setuptools.package-data]` and loaded with `importlib.resources` in
  `concplan.core.prompts`) contains the 4 active prompt assets:
  1. `scir_generation.md` — SCIR v4.0 generation (`lock_guard`, `while_wait`,
     `seq_hole`, `req`).
  2. `hole_repair.md` — Pure hole filling, Loop B violation repair, and Loop C
     escalation protocol.
  3. `cir_patch.md` — Single adjacent `mutex_lock` swap candidate format.
  4. `cir_local_revision.md` — Function-scoped local CIR revision format.
- **`benchmarks/`** holds two active suites:
  - `benchmarks/families/<family>/<case>/` — Clause-numbered `REQUIREMENTS.md`,
    `requirements.json`, frozen `contract.json`, and reference CIR/Rust files.
  - `benchmarks/real-cases/cases/<case>/` — Real-world upstream concurrency bugs
    with provenance, mapping notes, contracts, and reference CIR models.

---

## 4. Extending ConcPlanVerify

### 4.1 Plugging in a New `StaticVerifierPlugin`

Implement the `StaticVerifierPlugin` protocol from
`concplan.code_verify.static_verify` and pass it to
`CompositeStaticVerifier` (which threads `codegen_filled.json` across plugins
and preserves `unknown` / `fail_*` without ever downgrading to `pass`):

```python
from pathlib import Path
from concplan.code_verify import (
    AstHolePurityPlugin,
    CompositeStaticVerifier,
    SpnMirRefinementPlugin,
    StaticVerificationReport,
    StaticViolation,
)

class MyCustomCheckerPlugin:
    name = "my_custom_checker"

    def verify(
        self,
        *,
        skeleton_dir: Path,
        filled_dir: Path,
        spec_path: Path | None = None,
        contract_path: Path | None = None,
        codegen_map_path: Path | None = None,
        fills: dict[str, str] | None = None,
    ) -> StaticVerificationReport:
        # Inspect filled_dir / "src/main.rs" or invoke an external static analyzer.
        # Return status="pass" | "fail_hole" | "fail_refinement" | "unknown" | "error".
        return StaticVerificationReport(
            status="pass",
            plugin=self.name,
            checks={"custom_check": True},
        )

verifier = CompositeStaticVerifier([
    AstHolePurityPlugin(),
    SpnMirRefinementPlugin(client=client),
    MyCustomCheckerPlugin(),
])
```

Any violation carrying `hole_id` (or a `rust_span` inside a hole's line range)
is automatically routed to **Loop B** hole-local repair by `run_e2e_pipeline`;
non-hole violations escalate to **Loop C**.

### 4.2 Plugging in a New LLM Provider

1. Implement the `LlmClient` protocol (`chat(system_prompt, user_prompt, *, temperature=0.0, max_tokens=4096) -> tuple[str, dict[str, Any]]`) in `src/concplan/core/llm.py` and register its provider key in `create_llm_client` and `default_base_url`.
2. Wrap it with `LlmCandidateProvider(client, model_config, system_prompt=..., user_prompt_builder=...)` — or implement the `CandidateProvider` protocol directly for custom local/mock backends.
3. Pass your `cir_provider` and (optionally distinct) `hole_provider` to `run_e2e_pipeline(...)`.
