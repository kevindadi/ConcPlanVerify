# two_workers_shared_counter

Two worker threads each increment a shared counter once under a mutex; the main thread joins both workers. Workers also format a local log tag. The program must be deadlock-free and the counter must stay within 0..2.

## Entities
- **Roles / Functions**: `worker_a`, `worker_b`, `main`
- **Resources**: `counter_mutex`, `counter`
- **Domain Types**: `LogTag` (hole: Locally formatted short log tag string produced by each worker; pure sequential formatting logic.)

## Requirements
- **R1.** [V] (safety) The system must never deadlock.
- **R2.** [V] (safety) The shared counter must always stay within the range 0 to 2 inclusive.
- **R3.** [V] (safety) Each worker thread increments the shared counter exactly once under the mutex.
- **R4.** [V] (reachability) The main thread must eventually complete after both worker threads finish.
- **R5.** [U] (sequential_hole) Each worker formats a short local log tag while updating the counter.
