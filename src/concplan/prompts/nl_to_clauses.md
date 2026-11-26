# Stage 0.1: Natural Language to Grounded Atomic Clauses (`nl_to_clauses`)

You decompose a free-form natural-language concurrency specification into:

1. **Grounded Entities** (`roles`, `resources`, `domain_types`), and
2. **Numbered Atomic Requirement Clauses (`R1..Rn`)** classified into verifiable concurrency
   properties vs. unverifiable sequential/domain payload logic.

Output **only** a single JSON object.

## 1. Entity Grounding Rules

- **`roles`**: Concurrent actor / thread / worker function names (e.g., `"producer"`, `"consumer"`,
  `"reader"`, `"writer"`, `"worker_a"`, `"worker_b"`). Use short, valid Rust/CIR identifiers.
- **`resources`**: Shared synchronization primitives and modeled shared variables (e.g., `"book_lock"`,
  `"queue_mutex"`, `"count"`, `"ready"`, `"jobs_ch"`, `"permits"`).
- **`domain_types`**: Business/domain data structures whose internal fields and pure algorithms
  belong in a **`TypeHole`** (`src/domain.rs`), NOT in concurrency state (e.g., `OrderBook`,
  `MetricWindow`, `RoutingTable`). For each domain type, provide `name`, `description`, `kind`
  (`"hole"`, `"struct"`, or `"array"`), and linked `req` clause IDs.

## 2. Atomic Clause Classification Rules

Split compound requirements into atomic clauses numbered `R1`, `R2`, ..., `Rn`.
Assign every clause one `category` and boolean `verifiable`:

| `category` | `verifiable` | Meaning |
| --- | --- | --- |
| `"safety"` | `true` | Deadlock freedom, mutual exclusion, lock ordering, invariant bounds (`AG P`, `G P`). |
| `"liveness"` | `true` | Inevitable response, starvation freedom, progress under fairness (`AG(P -> AF Q)`, `G(P -> F Q)`). |
| `"reachability"` | `true` | Non-triviality, task completion, state reachability (`EF P`, `AG EF P`, `preserved`). |
| `"sequential_hole"` | `false` | Pure sequential algorithms, business payload fields/methods, formatting, or domain calculations (`[U]`). Delegated to `SeqHole` / `TypeHole` and NEVER turned into bogus model-checking properties. |

## 3. Exact Output Schema

```json
{
  "title": "order_book_rwlock",
  "summary": "Concurrent order book guarded by an RwLock with reader/writer workers.",
  "entities": {
    "roles": ["reader", "writer"],
    "resources": ["book_lock", "pending_orders"],
    "domain_types": [
      {
        "name": "OrderBook",
        "description": "In-memory bid/ask price-level book with pure match/insert methods",
        "kind": "hole",
        "req": ["R4"]
      }
    ]
  },
  "clauses": [
    {
      "id": "R1",
      "text": "The system must never deadlock.",
      "category": "safety",
      "verifiable": true,
      "entities": ["book_lock"]
    },
    {
      "id": "R2",
      "text": "Both reader and writer workers must be able to complete.",
      "category": "reachability",
      "verifiable": true,
      "entities": ["reader", "writer"]
    },
    {
      "id": "R3",
      "text": "Whenever pending_orders > 0, eventually pending_orders == 0.",
      "category": "liveness",
      "verifiable": true,
      "entities": ["pending_orders", "writer"]
    },
    {
      "id": "R4",
      "text": "OrderBook stores price levels and computes best-bid matching purely in memory.",
      "category": "sequential_hole",
      "verifiable": false,
      "entities": ["OrderBook"]
    }
  ]
}
```
