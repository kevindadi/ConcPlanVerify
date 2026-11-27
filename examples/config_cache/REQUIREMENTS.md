# config_cache

Concurrent in-memory configuration cache supporting shared reads of the current configuration snapshot, writes of new configurations, and archiving of pending changes, with thread safety and deadlock freedom.

## Entities
- **Roles / Functions**: `reader`, `writer`
- **Resources**: `cache_lock` (RwLock<ConfigSnapshot>), `pending_changes` (Var: Int[0..=1], init=0)
- **Domain Types**: `ConfigSnapshot` (hole: Pure in-memory configuration version and entry fields without strings, loops, or synchronization)

## Requirements
- **R1.** [V] (safety) The system must never deadlock under concurrent reader and writer execution.
- **R2.** [V] (safety) At all reachable states, pending_changes stays within 0..=1 (pending_changes <= 1).
- **R3.** [V] (liveness) Whenever pending_changes > 0 is reached, on all execution paths pending_changes eventually returns to 0.
- **R4.** [V] (reachability) Both reader and writer workers must be able to complete execution.
- **R5.** [U] (sequential_hole) Construct, inspect, and update ConfigSnapshot entries and pure integer version tags inside sequential/type holes without synchronization, loops, unsafe, or String formatting.
