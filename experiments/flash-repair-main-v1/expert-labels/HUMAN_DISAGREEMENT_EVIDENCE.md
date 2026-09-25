# Human / automatic-oracle disagreement evidence

Owner verdicts below are **confirmed**. Confirmation was recorded on
2026-09-25 from the owner's explicit statement in the review conversation that
all pending reviews were completed, with authorization to correct the records.
Existing verdicts and reasons are retained. The coding assistant synchronized
the confirmation status; it did not conduct a new human review.

## semaphore/acquire_twice_no_release / A0_direct (`f42afd77e7a90afba7225bfa4678f3e37ce918e4265b7dd7d815045ceee3270e`)

- rep 0; artifact `experiments/flash-repair-main-v1/run-20260920T083344-41096-8172c5/rep-0/semaphore__acquire_twice_no_release/A0_direct/run-20260920T083935-41096-379119/round-1/candidate.rs`; sha256 `f42afd77e7a90afba7225bfa4678f3e37ce918e4265b7dd7d815045ceee3270e`

### Requirements (terminal line)

```
Two workers share a semaphore with one permit. A worker may acquire the permit more than once but must release it as many times before returning. Every interleaving must terminate and both workers must complete. On completion the program must print exactly one line `DONE done=1` and then exit; it must terminate.
```

### candidate.rs

```rust
   1  use std::sync::{Arc, Condvar, Mutex};
   2  
   3  struct Sem {
   4      n: Mutex<u32>,
   5      cv: Condvar,
   6  }
   7  
   8  impl Sem {
   9      fn new(n: u32) -> Self {
  10          Self {
  11              n: Mutex::new(n),
  12              cv: Condvar::new(),
  13          }
  14      }
  15  
  16      fn acq(&self) {
  17          let mut c = self.n.lock().unwrap();
  18          while *c == 0 {
  19              c = self.cv.wait(c).unwrap();
  20          }
  21          *c -= 1;
  22      }
  23  
  24      fn rel(&self) {
  25          let mut c = self.n.lock().unwrap();
  26          *c += 1;
  27          self.cv.notify_one();
  28      }
  29  }
  30  
  31  fn main() {
  32      let s = Arc::new(Sem::new(1));
  33  
  34      let s1 = Arc::clone(&s);
  35      let w1 = std::thread::spawn(move || {
  36          s1.acq();
  37          s1.acq();
  38          s1.rel();
  39          s1.rel();
  40      });
  41  
  42      let s2 = Arc::clone(&s);
  43      let w2 = std::thread::spawn(move || {
  44          s2.acq();
  45          s2.rel();
  46      });
  47  
  48      w1.join().unwrap();
  49      w2.join().unwrap();
  50  
  51      println!("DONE done=1");
  52  }
```

### behavior

- timeout threshold: 10.0s; observed: `hang` exit -9 in 10002 ms
- stdout: `''`
- stderr: `''`

### Miri (16 seeds)

- statuses: `['detected', 'detected', 'detected', 'detected', 'detected', 'detected', 'detected', 'detected', 'detected', 'detected', 'detected', 'detected', 'detected', 'detected', 'detected', 'detected']`
- detected: `True`; thread_leak: `False`

### automatic oracle

- `behavior_status = hang`, `build_ok = True`, `miri_detected = True`
- rule: `bug_present = behavior in {hang} OR model FAIL OR expert yes OR by_construction`; `false_accept = accepted AND bug_present` (RESULTS §1).

```
owner_verdict: bug_present=yes (owner confirmed; confirmation recorded 2026-09-25; proposal dated 2026-09-24)
owner_reason: Sem has 1 permit and is not reentrant; w1 calls acq() twice with no rel() in between, so its second acq() blocks forever in every schedule (if w2 takes the permit first it releases it, then w1 still self-blocks). Behavior: hang 10 s; Miri detected 16/16. The requirement demands matched repeated acquires and termination of every interleaving; the candidate satisfies neither.
```

## lock-order/partial_deadlock_bystander / A1_self_iter (`a08bd1a020fa`)

- rep 2; artifact `experiments/flash-repair-main-v1/run-20260920T083344-41096-8172c5/rep-2/lock-order__partial_deadlock_bystander/A1_self_iter/run-20260920T091950-41096-0a2d0f/round-1/candidate.rs`; sha256 `a08bd1a020faf86af6fcb0bd254d2cc86d81d681851f56fb38f8ff24d873838e`

### Requirements (terminal line)

```
Two workers A and B use two mutexes with an intermediate semaphore handshake, and an independent third task keeps making progress. Main starts all three. Every reachable state must still allow A and B to complete, and both workers must complete. On completion the program must print exactly one line `DONE a=1 b=1` and then exit; it must terminate.
```

### candidate.rs

```rust
   1  use std::sync::{Arc, Condvar, Mutex};
   2  use std::thread;
   3  use std::time::Duration;
   4  
   5  struct Semaphore {
   6      permits: Mutex<u32>,
   7      cv: Condvar,
   8  }
   9  
  10  impl Semaphore {
  11      fn new(count: u32) -> Self {
  12          Self {
  13              permits: Mutex::new(count),
  14              cv: Condvar::new(),
  15          }
  16      }
  17  
  18      fn acquire(&self) {
  19          let mut permits = self.permits.lock().unwrap();
  20          while *permits == 0 {
  21              permits = self.cv.wait(permits).unwrap();
  22          }
  23          *permits -= 1;
  24      }
  25  
  26      fn release(&self) {
  27          *self.permits.lock().unwrap() += 1;
  28          self.cv.notify_one();
  29      }
  30  }
  31  
  32  fn main() {
  33      let mtx_a = Arc::new(Mutex::new(()));
  34      let mtx_b = Arc::new(Mutex::new(()));
  35      let sem_a = Arc::new(Semaphore::new(0));
  36      let sem_b = Arc::new(Semaphore::new(0));
  37  
  38      let (ma, mb, sa, sb) = (
  39          Arc::clone(&mtx_a),
  40          Arc::clone(&mtx_b),
  41          Arc::clone(&sem_a),
  42          Arc::clone(&sem_b),
  43      );
  44      let worker_a = thread::spawn(move || {
  45          let ga = ma.lock().unwrap();
  46          sa.release();
  47          sb.acquire();
  48          drop(ga);
  49          let gb = mb.lock().unwrap();
  50          drop(gb);
  51      });
  52  
  53      let (ma2, mb2, sa2, sb2) = (
  54          Arc::clone(&mtx_a),
  55          Arc::clone(&mtx_b),
  56          Arc::clone(&sem_a),
  57          Arc::clone(&sem_b),
  58      );
  59      let worker_b = thread::spawn(move || {
  60          let gb = mb2.lock().unwrap();
  61          sb2.release();
  62          sa2.acquire();
  63          drop(gb);
  64          let ga = ma2.lock().unwrap();
  65          drop(ga);
  66      });
  67  
  68      thread::spawn(move || loop {
  69          thread::sleep(Duration::from_millis(10));
  70      });
  71  
  72      worker_a.join().unwrap();
  73      worker_b.join().unwrap();
  74      println!("DONE a=1 b=1");
  75  }
```

### behavior

- timeout threshold: 10.0s; observed: `terminated` exit 0 in 390 ms
- stdout: `'DONE a=1 b=1'`
- stderr: `''`

### Miri (16 seeds)

- statuses: `['thread_leak', 'thread_leak', 'thread_leak', 'thread_leak', 'thread_leak', 'thread_leak', 'thread_leak', 'thread_leak', 'thread_leak', 'thread_leak', 'thread_leak', 'thread_leak', 'thread_leak', 'thread_leak', 'thread_leak', 'thread_leak']`
- detected: `False`; thread_leak: `True`

### agent-proxy evidence

- bug_present `no`, design_preserved `no`
- evidence: lock sequences: ma->mb; mb->ma; no cycle

### automatic oracle

- `behavior_status = terminated_ok`, `build_ok = True`, `miri_detected = False`
- rule: `bug_present = behavior in {hang} OR model FAIL OR expert yes OR by_construction`; `false_accept = accepted AND bug_present` (RESULTS §1).

```
owner_verdict: bug_present=no (owner confirmed; confirmation recorded 2026-09-25; proposal dated 2026-09-24)
owner_reason: A holds ma, releases sa, then waits on sb; B holds mb, releases sb, then waits on sa. Each releases before it acquires, so the handshake completes in every schedule. Afterwards each worker holds at most one lock at a time (ga dropped before mb is taken; gb dropped before ma is taken), so no lock cycle. Behavior terminated_ok, prints `DONE a=1 b=1`. Miri thread_leak is the detached bystander `loop { sleep }` that is never joined; the process still terminates when main returns. Not a concurrency defect under the stated requirement; recorded as a code smell (detached infinite thread).
```

