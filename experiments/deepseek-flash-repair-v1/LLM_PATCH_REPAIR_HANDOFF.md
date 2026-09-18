# LLM_PATCH_REPAIR_HANDOFF

Minimal, independently reviewable closed loop: **LLM proposes one constrained CIR
patch → Rust verifies the full frozen contract → real rejection feedback →
accept → independent replay**. Two frozen defect models were repaired by the live
model. This is a single-step-patch feasibility result on two samples, not an
effect-size study and not a source-level repair claim.

Repositories: Python `/Users/kevin/local-repos/ConcPlanVerify`; Rust
`/Users/kevin/local-repos/ConcIR`; evidence
`experiments/deepseek-flash-repair-v1/` (this directory).

## 1. The two reviewed gaps, fixed

### 1a. Task-level structural fidelity (P2)
`python/cir_workflow/structural.py` now checks the frozen **task templates**
instead of "any reversed locks anywhere":

- entry is exactly `main::main`;
- the entry starts exactly the two expected tasks with a single `scope`;
- each task's lock/unlock sequence is exactly `lock x, lock y, unlock y, unlock x`
  for the expected pair, with resource identity resolved by owner (FQN);
- for the cross-module task, `main` owns `a`, `other` owns `b`, both
  `requires.resources` declare the cross resource;
- structures outside the template (spawn/call instead of scope, two scopes,
  control flow in a task, ambiguous resources) are `ok: None` (unknown), never
  faithful.

Regressions (`python/tests/test_structural.py`): deleting a scope member,
an unspawned reversed function, early release of the first lock, and a
cross-module local-name shadow are all rejected; the faithful model passes.
The three previously delivered pilot CIRs still pass.

### 1b. Replay payload / artifact consistency (P2)
`concir_client._validate_replay_payload` is schema-aware and rejects
contradictions, not just empty/garbage output:

- integers must be real ints (a bool is rejected);
- for `concir-repair-artifact-v1`: `nodes` equals the artifact node count,
  `outcome` equals the artifact outcome, `chain_len` equals the patch-chain
  length, `input_outcome` equals the root report outcome, and the
  accepted node/flag are consistent with a repaired vs non-repaired outcome;
- for `concir-external-patch-artifact-v1`: one node/one edit, `accepted_ok`
  matches `accepted` and a `PASS` verification, `accepted_node` is `0` only when
  accepted, and `outcome` is the verification outcome mapped to a repair outcome.

Regression (`tests/test_concir_client.py::ConcirReplayContradictionTests`): the
review's contradictory payload (`nodes=999`, `accepted_ok=false`,
`outcome=no_acceptable_candidate` on a real search artifact) is rejected; a bool
count is rejected; a consistent payload is accepted.

The workflow success gate (`patch_repair._accepted_artifact_ok` /
`_replay_accepts`) requires this run's artifact, `accepted=true`, a complete
`PASS`, and a replay that reports `accepted_ok=true, accepted_node=0,
outcome=repaired`. A legitimately rejected artifact can replay successfully but
is not repair success.

## 2. Rust protocol (ConcIR)

New module `src/repair/external.rs`; `reports_match` made `pub(crate)`;
`repair/mod.rs` registers the module; `src/bin/concir-backend.rs` adds two
subcommands and schema-dispatches `replay`. Core verification/patch code is
reused unchanged; no new repair operation was added.

| command | input | output | notes |
| --- | --- | --- | --- |
| `repair-context <model> <contract> [--artifact f]` | frozen model + contract | `concir-repair-context-v1` | fingerprints, allowed scope, root report/diagnostics, functions with lock sids + `function_hash`; never a solved patch |
| `evaluate-patch <context> <candidate> [--artifact f]` | context + one candidate | `concir-external-patch-artifact-v1` | binds `context_fingerprint`, re-checks hash/scope/adjacency/distinct resources/non-control, applies, static-validates, lowers/supports, re-binds contract, full verify |
| `replay <artifact>` | any artifact | `ReplayResult` | external schema → `replay_external_patch`; v1 search schema → existing `replay_artifact` (unchanged) |

Candidate schema `concir-external-patch-candidate-v1`:
`{context_fingerprint, patch:{module, function, original_hash, changes:[{kind:"swap_statements",a,b}]}}`.
Exactly one change; deletion, whole-program replacement, contract changes, extra
edits and out-of-scope/stale targets are rejected by Rust (not by the prompt).

Exit codes: `evaluate-patch` 0 accepted; 1 verification FAIL; 3 UNKNOWN; 4
invalid/scope/hash/shape/static; 5 UNSUPPORTED; 2 protocol/parse. `replay` 0
replayed, 4 rejected (with `artifact replay failed: <reason>`).

Tamper resistance: replay rebuilds the context from the embedded inputs, checks
fingerprints, re-applies the patch, re-verifies, compares the stored report and
the chain, and rejects a contradictory `accepted`/`status`; see
`test_patch_repair.py::test_accepted_artifact_replays_and_tamper_fails`.

## 3. Python patch loop (ConcPlanVerify)

`python/cir_workflow/patch_repair.py` (`ExternalPatchRepairWorkflow`):
read frozen model/contract → Rust context → up to 3 rounds of
(propose one candidate → `evaluate-patch`) → on acceptance, Rust `replay`.
Round records keep the raw text, parsed candidate, evaluation status, real
rejection reason, verification outcome, duplicate flag and replay status.
Feedback is the actual structured reason (`render_patch_feedback`); Python never
rewrites the target/hash/sid or substitutes an enumerator answer, and saves the
raw response and the submitted candidate separately. Labels:
`candidate_source` (scripted/llm), `validator=tool`,
`repair_mode=external_single_patch`.

Prompt asset `prompt_assets/concir_patch_v1.md` (sha in `manifest.json`) gives the
schema, sids, hashes, the single allowed operation and the forbidden changes.
Provider entry points: `ScriptedPatchProvider` (offline) and
`RecordingPatchProvider` (live, wraps the Flash client with sanitized evidence).

CLI: `patch-repair` (offline scripted) and `live-repair` (live tasks; DeepSeek
Flash only, shared persisted budget).

## 4. Frozen inputs

Copied byte-for-byte from `deepseek-flash-pilot-v1`; hashes validated before any
patch; not regenerated.

| task | model sha256 | contract sha256 |
| --- | --- | --- |
| `r1_t2_abba` | `d2d4958ba7a5a7ac…` | `961458b818416972…` |
| `r2_t3_cross_module_abba` | `a513fadc58079ab1…` | `a53d94331df2a250…` |

Both roots reconfirmed `FAIL` complete on the new binary before patching;
structural fidelity `True` for both.

## 5. Offline results (feedback path)

`offline-demo/` (scripted, no key/network), one wrong-hash patch then one valid
patch per task:

| task | round 1 | round 2 | final |
| --- | --- | --- | --- |
| r1 | rejected `apply_error` | accepted + replayed | accepted |
| r2 | rejected `apply_error` | accepted + replayed | accepted |

This proves the real Rust rejection reason is carried into the next request and
that a later candidate is verified and replay-accepted — not merely asserted.

## 6. Live DeepSeek Flash results

Batch `runs/run-20260918T093903-97467-f2db10`:

| task | root | calls | round 1 | status | source | validator | mode |
| --- | --- | ---: | --- | --- | --- | --- | --- |
| `r1_t2_abba` | FAIL | 1 | accepted, replay replayed | accepted | llm | tool | external_single_patch |
| `r2_t3_cross_module_abba` | FAIL | 1 | accepted, replay replayed | accepted | llm | tool | external_single_patch |

- Real HTTP attempts: **2 / 6**; stop reason none; both responses
  `deepseek-flash`, `thinking` disabled, `finish_reason=stop`, usage present,
  ≤1.7 s each.
- Both tasks were accepted on round 1, so the **live rejection-feedback retry was
  not triggered**; the feedback path is covered by the offline scripted demos and
  tests, not by injecting fake model errors.
- No Pro/alias/fallback was used; no key or auth header was written to any
  record (161 evidence files scanned for the key string: none).

## 7. Tests

- Python: **56 tests / OK** (`python-tests.txt`), including the real-CLI
  integration and the two new regression groups.
- Rust: `INSTA_UPDATE=no cargo test --offline --all-targets --no-fail-fast`
  → **229 passed / 0 failed / 0 warnings** (`rust-tests.txt`).
- Old compatibility: the earlier v1 search artifacts (including the two pilot
  tool-repair artifacts) still replay via the schema-dispatched `replay`.

## 8. Reproduce

```bash
# build the current backend
cd /Users/kevin/local-repos/ConcIR && cargo build --release --offline --bin concir-backend

# offline scripted feedback demo (no key, no network)
cd /Users/kevin/local-repos/ConcPlanVerify
PYTHONPATH=python python3 -m cir_workflow --out <dir> patch-repair \
  --model <frozen.cir.json> --contract <contract.json> --script <responses.json>

# live Flash patch repair (key from ConcPlanVerify/.env; budget in REPAIR_TASKS.json)
PYTHONPATH=python python/.venv/bin/python -m cir_workflow \
  --out experiments/deepseek-flash-repair-v1/runs \
  --binary /Users/kevin/local-repos/ConcIR/target/release/concir-backend \
  live-repair --tasks experiments/deepseek-flash-repair-v1/REPAIR_TASKS.json
```

Build inputs: ConcIR binary sha256 `73b5dd64012e5c53…`; Rust commit
`d634f08`/`75b1882`; ConcPlanVerify commit `6f4aa05` plus this round's README /
CLI-docstring edits (working tree). Application/prompt/input hashes are in
`manifest.json`.

## 9. Limits

- Two samples, single-step adjacent mutex swaps only; no multi-patch
  accumulation, no other operation, no source-level patch claim.
- Both live repairs succeeded on the first proposal, so the live feedback loop is
  not exercised by the real model (only offline, deterministically).
- The models are the frozen issue-derived reductions from the previous pilot, not
  the upstream programs; the same abstraction caveats apply.
- No new provider, no agent framework, no benchmark scale-up, no Pro.
