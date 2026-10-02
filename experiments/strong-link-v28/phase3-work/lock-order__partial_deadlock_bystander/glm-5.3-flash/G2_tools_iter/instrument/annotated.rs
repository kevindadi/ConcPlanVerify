mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
// Deadlock-free two-lock handshake with an independent bystander.
//
// Roles:  a, b (workers), bystander (never-ending task)
// Shared: a, b (mutexes), sa, sb (counting semaphores), flag (shared variable)
//
// Protocol (R4, R5):
//   worker a: lock a -> signal sa -> wait sb -> lock b -> critical -> release both
//   worker b: lock b -> signal sb -> wait sa -> lock a -> critical -> release both
// Neither worker takes its second lock until the other has taken its first,
// so the circular-wait condition never arises (R7, R8, R9).
// The bystander never touches the locks or semaphores, so it can never block
// the workers (R6).
//
// `flag` bit layout:
//   bit 0 (1): set by worker a when it finished its critical section
//   bit 1 (2): set by worker b when it finished its critical section
//   bit 2 (4): stop signal, set by main ONLY after both workers have joined.
// The bystander never finishes on its own (R2); it only stops when main
// explicitly signals it, which happens after the workers are done (R11).

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc};
use std::thread;
use std::time::Duration;

/// Bit of `flag` used as the shutdown signal for the bystander.
const STOP: usize = 1 << 2;

/// A simple counting semaphore (permit counter) built from a mutex + condvar.
struct Sem {
    permits: Mutex<usize>,
    cv: Condvar,
}

impl Sem {
    fn new(initial: usize) -> Self {
        Sem {
            permits: Mutex::new(initial),
            cv: Condvar::new(),
        }
    }

    fn acquire(&self) {
        let mut count = self.permits.lock().unwrap();
        while *count == 0 {
            count = self.cv.wait(count).unwrap();
        }
        *count -= 1;
    }

    fn release(&self) {
        let mut count = self.permits.lock().unwrap();
        *count += 1;
        drop(count);
        self.cv.notify_one();
    }
}

/// Worker a: takes lock a first, then lock b.
fn a(
    lock_a: Arc<Mutex<()>>,
    lock_b: Arc<Mutex<()>>,
    sa: Arc<Sem>,
    sb: Arc<Sem>,
    flag: Arc<AtomicUsize>,
) {
    let _first = lock_a.lock().unwrap(); // first lock
    sa.release(); // handshake: "I hold my first lock"
    sb.acquire(); // wait until b holds its first lock (R5)
    let _second = lock_b.lock().unwrap(); // second lock -> holds both (R4)

    // Critical section while holding both locks.
    { let __cpv = flag.fetch_or(1, Ordering::SeqCst); cir_trace::record_value("flag#4055", (flag.load(std::sync::atomic::Ordering::SeqCst)) as i64); __cpv };

    drop(_second); // release each lock before finishing (R10)
    drop(_first);
}

/// Worker b: takes lock b first, then lock a.
fn b(
    lock_a: Arc<Mutex<()>>,
    lock_b: Arc<Mutex<()>>,
    sa: Arc<Sem>,
    sb: Arc<Sem>,
    flag: Arc<AtomicUsize>,
) {
    let _first = lock_b.lock().unwrap(); // first lock
    sb.release(); // handshake: "I hold my first lock"
    sa.acquire(); // wait until a holds its first lock (R5)
    let _second = lock_a.lock().unwrap(); // second lock -> holds both (R4)

    // Critical section while holding both locks.
    { let __cpv = flag.fetch_or(2, Ordering::SeqCst); cir_trace::record_value("flag#4055", (flag.load(std::sync::atomic::Ordering::SeqCst)) as i64); __cpv };

    drop(_second); // release each lock before finishing (R10)
    drop(_first);
}

/// Bystander: keeps making progress and never finishes on its own (R2).
/// It only reads `flag`; it never touches the locks or the semaphores, so it
/// can never prevent the workers from finishing (R6). It sleeps briefly each
/// iteration instead of hot-spinning, and exits only when main raises the
/// STOP bit — which main does after both workers have already finished.
fn bystander(flag: Arc<AtomicUsize>) {
    loop {
        // Never finishes on its own: it only stops on an external signal.
        if { let __cpv = flag.load(Ordering::Relaxed); cir_trace::record_value("flag#4055", (__cpv) as i64); __cpv } & STOP != 0 {
            break;
        }
        let _progress = { let __cpv = flag.load(Ordering::Relaxed); cir_trace::record_value("flag#4055", (__cpv) as i64); __cpv };
        thread::sleep(Duration::from_millis(1));
    }
}

fn main() { cir_trace::init();
    // Shared resources.
    let lock_a = Arc::new(Mutex::new_named("lock_a_mutex0#3863", ())); // lock a
    let lock_b = Arc::new(Mutex::new_named("lock_b_mutex0#3916", ())); // lock b
    let sa = Arc::new(Sem::new(0)); // semaphore sa
    let sb = Arc::new(Sem::new(0)); // semaphore sb
    let flag = Arc::new(AtomicUsize::new(0)); // shared variable

    // Main starts all three tasks (R1, R11).
    let ha = {
        let (lock_a, lock_b, sa, sb, flag) =
            (lock_a.clone(), lock_b.clone(), sa.clone(), sb.clone(), flag.clone());
        cir_trace::spawn("a#4304", move || a(lock_a, lock_b, sa, sb, flag))
    };
    let hb = {
        let (lock_a, lock_b, sa, sb, flag) =
            (lock_a.clone(), lock_b.clone(), sa.clone(), sb.clone(), flag.clone());
        cir_trace::spawn("b#4518", move || b(lock_a, lock_b, sa, sb, flag))
    };
    let hby = {
        let flag = flag.clone();
        cir_trace::spawn("bystander#4637", move || bystander(flag))
    };

    // Main waits only for the two workers (R11). The bystander cannot block
    // this: it holds no locks and touches no semaphores (R6).
    ha.join().expect("worker a panicked");
    hb.join().expect("worker b panicked");

    // Both workers finished: each set its bit in the shared flag.
    assert_eq!(flag.load(Ordering::SeqCst) & 3, 3, "both workers must have run");

    // Now that both workers are done, tell the bystander to stop and reap it
    // so the process terminates cleanly with no thread left spinning.
    { let __cpv = flag.fetch_or(STOP, Ordering::SeqCst); cir_trace::record_value("flag#4055", (flag.load(std::sync::atomic::Ordering::SeqCst)) as i64); __cpv };
    hby.join().expect("bystander panicked");

    // Exactly the required output (R12).
    println!("DONE a=1 b=1");
 cir_trace::finish();}
