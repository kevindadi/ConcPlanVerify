# concurrent_access_stats

Concurrent access statistics supporting multiple concurrent request workers that record visit counts and generate visit summaries, ensuring accurate counts and deadlock freedom.

## Entities
- **Roles / Functions**: `recorder_a`, `recorder_b`
- **Resources**: `stats_lock` (Mutex<VisitSummary>, init=0), `visit_count` (Var: Int[0..=2], init=0), `summary_ready` (Var: Bool, init=false)
- **Domain Types**: `VisitSummary` (hole: Pure in-memory visit summary payload with integer count and boolean flags, no synchronization or strings)

## Requirements
- **R1.** [V] (safety) The system must never deadlock under concurrent recorder_a and recorder_b execution.
- **R2.** [V] (safety) At all reachable states, visit_count stays within 0..=2 (visit_count <= 2).
- **R3.** [V] (liveness) Whenever a visit is recorded (visit_count > 0), on all execution paths summary_ready eventually becomes true.
- **R4.** [V] (reachability) Both recorder_a and recorder_b workers must be able to complete execution.
- **R5.** [U] (sequential_hole) Construct, inspect, and update VisitSummary entries and pure integer visit counts inside sequential/type holes without synchronization, loops, unsafe, or String formatting.
