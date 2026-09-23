Looking at this program, I need to analyze the handshake protocol between workers A and B.

**Worker A:**
1. Lock `mtx_a`, then release `sem_a` (while holding `mtx_a`)
2. Acquire `sem_b`
3. Lock `mtx_b`

**Worker B:**
1. Lock `mtx_b`, then release `sem_b` (while holding `mtx_b`)
2. Acquire `sem_a`
3. Lock `mtx_a`

Let me trace the interleaving:

- A locks `mtx_a`, releases `sem_a` (permits_a = 1), unlocks `mtx_a`.
- B locks `mtx_b`, releases `sem_b` (permits_b = 1), unlocks `mtx_b`.
- A acquires `sem_b` (permits_b = 0), locks `mtx_b` — succeeds since B released it.
- B acquires `sem_a` (permits_a = 0), locks `mtx_a` — succeeds since A released it.

Both complete. Now consider the ordering where A runs fully first:

- A locks `mtx_a`, releases `sem_a` (permits_a = 1), unlocks `mtx_a`.
- A acquires `sem_b` — but `sem_b` has 0 permits! A blocks waiting for B.
- B locks `mtx_b`, releases `sem_b` (permits_b = 1), unlocks `mtx_b`.
- B acquires `sem_a` (permits_a = 1 → 0), locks `mtx_a` — succeeds.
- B completes. A wakes up, acquires `sem_b`, locks `mtx_b` — succeeds.

Both complete. The semaphores are used as a rendezvous: each worker signals it has passed its first critical section, and waits for the other to do the same. Since each worker releases its semaphore *before* acquiring the other's, and the release happens while holding the lock (but the lock is released right after), there's no deadlock.

Let me check the critical concern: does A hold `mtx_a` while waiting on `sem_b`? No — A releases `mtx_a` (the block ends) before calling `sb.acquire()`. Similarly B releases `mtx_b` before `sa.acquire()`. So neither worker holds a mutex while blocking on a semaphore.

The semaphore itself: `acquire` uses a `while` loop with `wait`, correctly handling spurious wakeups. `release` increments and notifies. No lost wakeup because the increment happens under the lock before notify.

The third task just loops sleeping — it makes progress and never blocks on anything shared.

Both workers must complete: A needs `sem_b` (released by B) and B needs `sem_a` (released by A). Both releases happen unconditionally before the acquires, so both will always be satisfied. No deadlock, no lost wakeup, no data race (all shared state is behind mutexes/condvar).

The program terminates and prints exactly one line.

NO_ISSUES
