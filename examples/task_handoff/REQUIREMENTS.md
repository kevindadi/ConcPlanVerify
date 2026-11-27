# task_producer_consumer_handoff

Concurrent task production and consumption handoff: an upstream producer generates pending task data, a downstream consumer waits when no tasks are available, computes a checksum result upon receiving a task, with no task loss, no deadlock, and eventual processing of all submitted tasks.

## Entities
- **Roles / Functions**: `producer`, `consumer`
- **Resources**: `task_channel` (Channel<TaskPayload>, init=0), `pending_tasks` (Var: Int[0..=2], init=0), `processed_tasks` (Var: Int[0..=2], init=0)
- **Domain Types**: `TaskPayload` (hole: Pure in-memory task data with integer identifier and payload fields, no synchronization or strings), `ChecksumResult` (hole: Pure computed checksum result derived from TaskPayload, integer/boolean fields only)

## Requirements
- **R1.** [V] (safety) The system must never deadlock under concurrent producer and consumer execution.
- **R2.** [V] (safety) At all reachable states, pending_tasks stays within 0..=2 (pending_tasks <= 2).
- **R3.** [V] (safety) At all reachable states, processed_tasks stays within 0..=2 (processed_tasks <= 2).
- **R4.** [V] (liveness) Whenever pending_tasks > 0 is reached, on all execution paths pending_tasks eventually returns to 0 (all submitted tasks are eventually processed).
- **R5.** [V] (reachability) Both producer and consumer workers must be able to complete execution.
- **R6.** [U] (sequential_hole) Construct TaskPayload entries and compute ChecksumResult from them inside sequential/type holes without synchronization, loops, unsafe, or String formatting.
