#!/usr/bin/env python3
"""Schedule witness for the fixed condvar source. No model requests.

The hook delays the waiters until the notifier has called notify_all.
That order is already possible in the original program: g12 starts at 2,
so the notifier's two acquires do not wait for the waiters. Removing the
hook leaves that order in the original schedule set.
"""

from __future__ import annotations

import difflib
import hashlib
import os
import subprocess
import textwrap
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
NOTES = Path("/Users/kevin/paper-review/papers/ConcPlanVerify/notes")
SOURCE = (NOTES / "strong-link-v5/reexec-gpt6luna/GPT 6 Luna"
          / "condvar__notify_one_multi_waiter_wrong_pick/rep0/source.rs")
OUT = NOTES / "strong-link-v16" / "r6-witness"


HOOKS = textwrap.dedent("""
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
static GATE: AtomicBool = AtomicBool::new(false);
static STEP: AtomicUsize = AtomicUsize::new(0);
fn witness(label: &str) {
    let step = STEP.fetch_add(1, Ordering::SeqCst);
    eprintln!("WITNESS {step} {label}");
}
""")


def _insert(src: str) -> str:
    src = src.replace("use std::thread;\n", "use std::thread;\n" + HOOKS, 1)
    src = src.replace(
        "fn w1(m: Arc<Mutex<bool>>, cv: Arc<Condvar>, g12: Arc<Semaphore>, gN: Arc<Semaphore>) {\n",
        "fn w1(m: Arc<Mutex<bool>>, cv: Arc<Condvar>, g12: Arc<Semaphore>, gN: Arc<Semaphore>) {\n"
        "    while !GATE.load(Ordering::SeqCst) { std::thread::yield_now(); }\n"
        "    witness(\"w1_entered\");\n",
        1)
    src = src.replace(
        "fn w2(m: Arc<Mutex<bool>>, cv: Arc<Condvar>, g12: Arc<Semaphore>, gN: Arc<Semaphore>) {\n",
        "fn w2(m: Arc<Mutex<bool>>, cv: Arc<Condvar>, g12: Arc<Semaphore>, gN: Arc<Semaphore>) {\n"
        "    while !GATE.load(Ordering::SeqCst) { std::thread::yield_now(); }\n"
        "    witness(\"w2_entered\");\n",
        1)
    src = src.replace(
        "    while !*proceed {\n",
        "    witness(\"waiter_before_wait_loop\");\n    while !*proceed {\n")
    src = src.replace(
        "    cv.notify_all();\n",
        "    cv.notify_all();\n    witness(\"notifier_notify_all\");\n    GATE.store(true, Ordering::SeqCst);\n",
        1)
    return src


def main() -> int:
    original = SOURCE.read_text(encoding="utf-8")
    hooked = _insert(original)
    if hooked == original:
        raise SystemExit("hook was not inserted")
    OUT.mkdir(parents=True, exist_ok=True)
    (OUT / "original.rs").write_text(original, encoding="utf-8")
    (OUT / "hooked.rs").write_text(hooked, encoding="utf-8")
    diff = "".join(difflib.unified_diff(
        original.splitlines(keepends=True), hooked.splitlines(keepends=True),
        fromfile="original.rs", tofile="hooked.rs"))
    (OUT / "hook.diff").write_text(diff, encoding="utf-8")
    (OUT / "HASHES.txt").write_text(
        "original_sha256=" + hashlib.sha256(original.encode()).hexdigest() + "\n"
        "hooked_sha256=" + hashlib.sha256(hooked.encode()).hexdigest() + "\n",
        encoding="utf-8")
    work = OUT / "build"
    if work.exists():
        import shutil
        shutil.rmtree(work)
    (work / "src").mkdir(parents=True)
    (work / "src" / "main.rs").write_text(hooked, encoding="utf-8")
    sync = REPO / "runtime/concir_sync"
    (work / "Cargo.toml").write_text(
        "[package]\nname = \"r6witness\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"
        "[[bin]]\nname = \"r6witness\"\npath = \"src/main.rs\"\n"
        f"[dependencies]\nconcir_sync = {{ path = \"{sync}\" }}\n",
        encoding="utf-8")
    env = dict(os.environ)
    env.pop("CARGO_TARGET_DIR", None)
    build = subprocess.run(["cargo", "build", "--offline", "--quiet"], cwd=work, env=env,
                           capture_output=True, text=True)
    (OUT / "build.log").write_text(build.stdout + build.stderr, encoding="utf-8")
    if build.returncode != 0:
        print(build.stderr)
        return build.returncode
    run = subprocess.run([str(work / "target/debug/r6witness")], cwd=work, env=env,
                         capture_output=True, text=True, timeout=8)
    (OUT / "stdout.log").write_text(run.stdout, encoding="utf-8")
    (OUT / "witness.log").write_text(run.stderr, encoding="utf-8")
    print(run.stderr)
    print("return", run.returncode)
    order = [line.split()[2] for line in run.stderr.splitlines() if line.startswith("WITNESS")]
    notify = order.index("notifier_notify_all")
    waits = [i for i, name in enumerate(order) if name == "waiter_before_wait_loop"]
    if not (len(waits) == 2 and notify < min(waits)):
        raise SystemExit(f"witness order was {order}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
