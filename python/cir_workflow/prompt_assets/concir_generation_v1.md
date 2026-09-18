# ConcIR modular generation prompt (v1)

You write one **ConcIR** program as a single JSON object. CIR is the input to a
separate Rust verifier; you never verify, translate, repair or accept anything
yourself.

Output only the JSON object. No prose, no markdown fences.

## Top-level program

```json
{
  "program": "short_name",
  "version": "3.5.0",
  "entry": "main::main",
  "modules": [ /* one or more modules */ ]
}
```

`entry` is an FQN `module::function`. `version` is the schema version string
`"3.5.0"`.

## Module

```json
{
  "name": "main",
  "provides": {"resources": ["a"], "functions": ["main", "worker"]},
  "requires": {"resources": [], "functions": ["other::helper"]},
  "resources": [ /* resource declarations owned by this module */ ],
  "protection": [],
  "functions": [ /* function definitions */ ]
}
```

- `provides` lists resources/functions this module declares.
- `requires` lists FQNs of resources/functions owned by another module. Only
  list what the module actually uses; `mutex_lock` on `"other::b"` requires
  `requires.resources` to contain `"other::b"`.
- All resource and function names are unique across the whole program (the FQN
  is `module::name`).

## Resource

```json
{"name": "mtx", "kind": "sync", "type": "Mutex", "mode": "Sync"}
```

Supported `(kind, type)` pairs: `var`/`Var`, `var`/`Atomic`, `sync`/`Mutex`,
`sync`/`Semaphore`, `sync`/`Channel`, `sync`/`Condvar`. (`RwLock` is not
supported by the current verifier; do not emit it.) Semaphore/Channel carry
`count`/capacity fields from the schema.

## Function

```json
{
  "name": "worker",
  "kind": "normal",
  "form": "closure",
  "body": [ /* statements, each with a unique "sid" inside this function */ ]
}
```

`main` is `{"name": "main", "kind": "normal", "body": [{"sid": "s1", "kind": "scope", "funcs": ["worker"]}, {"sid": "s2", "kind": "return"}]}`.

## Statements

Every statement is an object with `"sid"` (unique within the function) and
`"kind"`. Use only these kinds:

| kind | fields |
| --- | --- |
| `mutex_lock` | `resource` |
| `mutex_unlock` | `resource` |
| `semaphore_acquire` / `semaphore_release` | `resource`, optional `count` |
| `channel_send` | `channel`, `value` |
| `channel_recv` | `channel`, `dst` |
| `condvar_wait` | `condvar`, `lock` |
| `condvar_notify` / `condvar_notify_all` | `condvar` |
| `atomic_load` / `atomic_store` / `atomic_cas` | `resource`, plus `dst`/`value`/`expected`/`desired`/`dst` |
| `read_shared` / `write_shared` | `resource`, plus optional `dst` / `expr` |
| `assign_local` | `target`, `expr` |
| `abstract_step` | optional `reads`, `writes`, `desc` |
| `call` | `func` (FQN), optional `args`, optional `dst` |
| `spawn` | `func` (FQN), optional `args`, `handle` |
| `scope` | `funcs` (list of FQNs) — spawn each and join all |
| `join` | `handle` |
| `goto` / `branch` / `switch` | control targets by `sid` |
| `return` | optional `value` |

Rules:

- A `mutex_lock` must be paired with a `mutex_unlock` of the same resource
  later in the same function on every path.
- `scope` members run concurrently; model one task as one function.
- Control-flow targets (`goto`/`branch`/`switch`) reference `sid`s in the same
  function.
- Do not invent fields or kinds. Do not add comments.

## Modelling rules

- One function per concurrent task; `main` wires tasks together with `scope`.
- Use shared resources for synchronization; do not serialize unrelated work.
- The program is checked against a caller-supplied contract, so you do not
  write or change the contract.
