mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
// R1: one main thread plus two short-lived workers (a and b).
// R2: an independent bystander task runs forever, making progress, never finishing.
// R3: workers a and b share locks `a` and `b`, and coordinate via permits `sa` and `sb`.
// R4: each worker holds both locks simultaneously inside its critical section.
// R5: handshake via the permits: a worker only takes its second lock after the
//     other worker has taken (and signaled) its first lock.
// R6: the bystander never touches the locks or permits, so it cannot block the workers.
// R7/R8: because each worker signals its own permit *before* waiting on the other's,
//     there is no circular wait; either worker can always finish.
// R9: no worker waits forever: each permit is signaled unconditionally by the other worker.
// R10: each worker drops both lock guards before returning.
// R11: main joins only the two workers; the bystander is left running.
// R12: main prints exactly `DONE a=1 b=1` and the program exits.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc};
use std::thread;

// A simple counting semaphore built from a mutex and a condition variable.
struct Semaphore {
    count: Mutex<usize>,
    cv: Condvar,
}

impl Semaphore {
    fn new(initial: usize) -> Self {
        Semaphore {
            count: Mutex::new(initial),
            cv: Condvar::new(),
        }
    }

    // Wait (P): block until the count is positive, then decrement.
    fn wait(&self) {
        let mut count = self.count.lock().unwrap();
        while *count == 0 {
            count = self.cv.wait(count).unwrap();
        }
        *count -= 1;
    }

    // Signal (V): increment the count and wake one waiter.
    fn signal(&self) {
        let mut count = self.count.lock().unwrap();
        *count += 1;
        self.cv.notify_one();
    }
}

fn main() { cir_trace::init();
    // Shared resources: locks a and b, permits sa and sb, and the shared flag.
    let lock_a: Arc<Mutex<()>> = Arc::new(Mutex::new_named("lock_a_mutex0#1994", ()));
    let lock_b: Arc<Mutex<()>> = Arc::new(Mutex::new_named("lock_b_mutex0#2053", ()));
    let sa: Arc<Semaphore> = Arc::new(Semaphore::new(0));
    let sb: Arc<Semaphore> = Arc::new(Semaphore::new(0));
    let flag: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));

    // R2: bystander task — keeps making progress (bumping `flag`) and never
    // finishes on its own. It touches no locks or permits (R6).
    {
        let flag = Arc::clone(&flag);
        thread::spawn(move || loop {
            { let __cpv = flag.fetch_add(1, Ordering::Relaxed); cir_trace::record_value("flag#2212", (flag.load(std::sync::atomic::Ordering::SeqCst)) as i64); __cpv };
            // Never returns; the process exit ends it (R11).
        });
    }

    // Worker a: takes lock a first, signals sa, waits for sb, then takes lock b.
    let worker_a = {
        let lock_a = Arc::clone(&lock_a);
        let lock_b = Arc::clone(&lock_b);
        let sa = Arc::clone(&sa);
        let sb = Arc::clone(&sb);
        cir_trace::spawn("worker_a#2863", move || {
            // First lock.
            let guard_a = lock_a.lock().unwrap();
            // Handshake: tell b that a has taken its first lock (R5).
            sa.signal();
            // Wait until b has taken its first lock before taking the second (R5).
            sb.wait();
            // Second lock — now both locks are held (R4).
            let guard_b = lock_b.lock().unwrap();
            // Critical section with both locks held.
            // Release each lock before finishing (R10).
            drop(guard_b);
            drop(guard_a);
        })
    };

    // Worker b: takes lock b first, signals sb, waits for sa, then takes lock a.
    let worker_b = {
        let lock_a = Arc::clone(&lock_a);
        let lock_b = Arc::clone(&lock_b);
        let sa = Arc::clone(&sa);
        let sb = Arc::clone(&sb);
        cir_trace::spawn("worker_b#3725", move || {
            // First lock.
            let guard_b = lock_b.lock().unwrap();
            // Handshake: tell a that b has taken its first lock (R5).
            sb.signal();
            // Wait until a has taken its first lock before taking the second (R5).
            sa.wait();
            // Second lock — now both locks are held (R4).
            let guard_a = lock_a.lock().unwrap();
            // Critical section with both locks held.
            // Release each lock before finishing (R10).
            drop(guard_a);
            drop(guard_b);
        })
    };

    // R11: main starts all three tasks (bystander above, workers here) and
    // finishes only after both workers have finished. The bystander is not joined.
    worker_a.join().unwrap();
    worker_b.join().unwrap();

    // R12: print exactly this line, then exit.
    println!("DONE a=1 b=1");
 cir_trace::finish();}
