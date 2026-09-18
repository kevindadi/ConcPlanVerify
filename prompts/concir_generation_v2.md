# ConcIR modular generation prompt (v2)

You write one **ConcIR** program as a single JSON object. CIR is the input
to a separate Rust verifier; you never verify, translate, repair or accept
anything yourself. Output only the JSON object, no prose or fences.

The statement and resource tables below are generated from the verifier's
machine schema (`concir-backend schema`); use exactly these kinds and field
names. `sid` is per-function and must match `^s[0-9]+$`.

## Statements

| kind | required fields | optional fields |
| --- | --- | --- |
| `assign_local` | `target`, `expr` | — |
| `atomic_cas` | `resource`, `expected`, `desired`, `dst` | — |
| `atomic_load` | `resource`, `dst` | — |
| `atomic_store` | `resource`, `value` | — |
| `branch` | `cond`, `then`, `else` | — |
| `call` | `func` | `args`, `dst` |
| `channel_recv` | `channel`, `dst` | — |
| `channel_send` | `channel`, `value` | — |
| `condvar_notify` | `condvar` | — |
| `condvar_notify_all` | `condvar` | — |
| `condvar_wait` | `condvar`, `lock` | — |
| `goto` | `target` | — |
| `join` | `handle` | — |
| `mutex_lock` | `resource` | — |
| `mutex_unlock` | `resource` | — |
| `nop` | — | — |
| `read_shared` | `resource` | `dst` |
| `return` | — | `value` |
| `scope` | `funcs` | — |
| `semaphore_acquire` | `resource` | `count` |
| `semaphore_release` | `resource` | `count` |
| `spawn` | `func`, `handle` | `args` |
| `switch` | `var`, `cases`, `default` | — |
| `write_shared` | `resource`, `expr` | — |

Every statement also carries `sid` (a string matching `^s[0-9]+$`).

## Resources

| type | kind/type fields | required | optional |
| --- | --- | --- | --- |
| `Atomic` | `kind='var'`, `type='Atomic'` | `name`, `kind`, `type`, `base`, `init` | — |
| `Channel` | `kind='sync'`, `mode='Sync'`, `type='Channel'` | `name`, `kind`, `type`, `mode`, `base`, `capacity` | — |
| `Condvar` | `kind='sync'`, `mode='Sync'`, `type='Condvar'` | `name`, `kind`, `type`, `mode` | — |
| `Mutex` | `kind='sync'`, `mode='Sync'`, `type='Mutex'` | `name`, `kind`, `type`, `mode` | — |
| `Semaphore` | `kind='sync'`, `mode='Sync'`, `type='Semaphore'` | `name`, `kind`, `type`, `mode` | `count` |
| `Var` | `kind='var'`, `type='Var'` | `name`, `kind`, `type`, `base`, `init` | — |

`expr`, `cond`, `value`, `expected`, `desired` are **JSON strings** holding a
source expression, never objects. Resource references use FQNs (`module::name`).

## Three minimal fragments

A variable resource (note `base` and `init` are required):

```json
{"name": "ready", "kind": "var", "type": "Var", "base": "Bool", "init": false}
```

A write and a predicate-guarded wait loop (control targets are `sid`s):

```json
{"sid": "s1", "kind": "write_shared", "resource": "main::ready", "expr": "true"}
{"sid": "s2", "kind": "branch", "cond": "ready == true", "then": "s4", "else": "s3"}
{"sid": "s3", "kind": "condvar_wait", "condvar": "main::cv", "lock": "main::m"}
{"sid": "s4", "kind": "goto", "target": "s2"}
```

A scope that starts two tasks and waits:

```json
{"sid": "s1", "kind": "scope", "funcs": ["main::t1", "main::t2"]}
```

## Top-level, module, function

```json
{ "program": "short_name", "version": "3.5.0", "entry": "main::main",
  "modules": [ { "name": "main",
    "provides": { "resources": ["m", "ready"], "functions": ["main", "t1"] },
    "requires": { "resources": [], "functions": [] },
    "resources": [ /* resource objects */ ],
    "protection": [ { "var": "ready", "lock": "m" } ],
    "functions": [ /* function objects */ ] } ] }
```

A function is `{"name": "t1", "kind": "normal", "form": "closure",
"body": [ ...statements... ]}`. A condition variable is paired with the
mutex passed as `lock`; a `mutex_lock` must be matched by a `mutex_unlock` on
every path. Do not emit unsupported kinds (`rwlock_*`, `select`, `async_call`,
`await`, `abstract_step`, `seq_hole`).

