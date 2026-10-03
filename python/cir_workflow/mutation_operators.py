"""Operation-bound mutations of mechanically generated Rust. No LLM calls."""
import re

LOCK_RE = re.compile(r'^\s*_g_\w+ = Some\(crate::cir_trace::lock\(&shared\.\w+, tag, "[^"]*", "[^"]*"\)\);\s*$')
UNLOCK_RE = re.compile(r'^\s*crate::cir_trace::unlock\(_g_\w+\.take\(\), tag, "[^"]*", "[^"]*"\);\s*$')
NOTIFY_RE = re.compile(r'crate::cir_trace::notify_(one|all)\(')
CHAN_RE = re.compile(r'\.(send_sid|recv_sid)\(')
SPAWN_RE = re.compile(r'^\s*__hs\.push\((crate::cir_trace::spawn|std::thread::spawn)\(')
SCOPE_RE = re.compile(r'crate::cir_trace::scope\(')


def _function_spans(lines):
    spans = []
    for i, l in enumerate(lines):
        if not re.search(r"\bfn\s+\w+\s*\([^)]*\)\s*\{", l):
            continue
        depth = l.count("{") - l.count("}")
        j = i + 1
        while j < len(lines) and depth > 0:
            depth += lines[j].count("{") - lines[j].count("}")
            j += 1
        spans.append((i, j))
    return spans


def mutants(src: str):
    lines = src.splitlines()
    out = []

    def emit(op, desc, new_lines):
        out.append((op, desc, "\n".join(new_lines) + "\n"))

    lock_idx = [i for i, l in enumerate(lines) if LOCK_RE.match(l)]
    for start, end in _function_spans(lines):
        in_fn = [i for i in lock_idx if start <= i < end]
        if len(in_fn) >= 2:
            i, j = in_fn[0], in_fn[1]
            nl = list(lines)
            nl[i], nl[j] = nl[j], nl[i]
            emit("M1", f"swap locks at {i+1},{j+1}", nl)
            break

    unlock_idx = [i for i, l in enumerate(lines) if UNLOCK_RE.match(l)]
    if unlock_idx:
        nl = list(lines)
        del nl[unlock_idx[0]]
        emit("M2", f"delete unlock at {unlock_idx[0]+1}", nl)

    if "notify_all(" in src or "notify_one(" in src:
        nl = src.replace("notify_all(", "notify_one(") if "notify_all(" in src \
            else src.replace("notify_one(", "notify_all(")
        emit("M4", "flip notify", nl.splitlines())

    for i, l in enumerate(lines):
        if CHAN_RE.search(l) and i > 0:
            nl = list(lines)
            nl[i - 1], nl[i] = nl[i], nl[i - 1]
            emit("M5", f"move channel op at {i+1} one statement earlier", nl)
            break

    for i, l in enumerate(lines):
        if NOTIFY_RE.search(l) or CHAN_RE.search(l) or SCOPE_RE.search(l):
            nl = list(lines)
            del nl[i]
            emit("M6", f"delete sync call at {i+1}", nl)
            break

    spawn_idx = [i for i, l in enumerate(lines) if SPAWN_RE.match(l)]
    if len(spawn_idx) >= 2:
        i, j = spawn_idx[0], spawn_idx[1]
        nl = list(lines)
        nl[i], nl[j] = nl[j], nl[i]
        emit("M7", f"swap spawn at {i+1},{j+1}", nl)

    resources = sorted(set(re.findall(r'crate::cir_trace::lock\(&shared\.\w+, tag, "([^"]+)"', src)))
    if lock_idx and len(resources) >= 2:
        i = lock_idx[0]
        old = re.search(r'lock\(&shared\.\w+, tag, "([^"]+)"', lines[i]).group(1)
        newr = next(r for r in resources if r != old)
        nl = list(lines)
        nl[i] = lines[i].replace(f'"{old}"', f'"{newr}"')
        emit("M8", f"lock at {i+1}: {old} -> {newr}", nl)

    return out


