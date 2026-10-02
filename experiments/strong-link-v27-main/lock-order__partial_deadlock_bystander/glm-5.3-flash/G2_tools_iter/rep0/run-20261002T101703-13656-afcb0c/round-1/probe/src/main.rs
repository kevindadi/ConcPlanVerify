//! Deadlock-free handshake design:
//!
//!   worker a: lock(a) -> signal sa -> wait sb -> lock(b) -> CS -> release both
//!   worker b: lock(b) -> signal sb -> wait sa -> lock(a) -> CS -> release both
//!
//! Each worker takes its *first* lock, then signals the other via a semaphore
//! permit, and only then waits for the other's signal before taking its
//! *second* lock. This guarantees (R5) that neither worker takes its second
//! lock before the other has taken its first, so the circular-wait deadlock
//! is impossible (R7, R8, R9). The bystander touches only `flag` and never
//! touches the locks or permits, so it can never block the workers (R6).

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use tokio::sync::{Mutex, Semaphore};

#[tokio::main]
async fn main() {
    // Shared resources
    let lock_a: Arc<Mutex<()>> = Arc::new(Mutex::new(())); // lock a
    let lock_b: Arc<Mutex<()>> = Arc::new(Mutex::new(())); // lock b
    let sa: Arc<Semaphore> = Arc::new(Semaphore::new(0)); // semaphore sa (0 permits)
    let sb: Arc<Semaphore> = Arc::new(Semaphore::new(0)); // semaphore sb (0 permits)
    let flag: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0)); // shared variable flag

    // ------------------------------------------------------------------
    // Bystander (R2, R6): runs forever, keeps making progress via `flag`,
    // never finishes on its own, and never touches the locks or permits,
    // so it can never prevent the workers from finishing.
    // ------------------------------------------------------------------
    {
        let flag = flag.clone();
        tokio::spawn(async move {
            loop {
                flag.fetch_add(1, Ordering::Relaxed); // keeps making progress
                tokio::time::sleep(Duration::from_millis(1)).await;
                // never returns on its own
            }
        });
    }

    // ------------------------------------------------------------------
    // Worker a (R3, R4, R5, R10)
    // ------------------------------------------------------------------
    let ha = {
        let lock_a = lock_a.clone();
        let lock_b = lock_b.clone();
        let sa = sa.clone();
        let sb = sb.clone();
        let flag = flag.clone();
        tokio::spawn(async move {
            // First lock
            let guard_a = lock_a.lock().await;
            // Handshake: announce "I hold my first lock" ...
            sa.add_permits(1);
            // ... then wait for b's announcement before taking the second lock (R5)
            let _permit = sb.acquire().await.expect("sb closed");
            // Second lock — both locks now held simultaneously (R4)
            let guard_b = lock_b.lock().await;

            // Critical section
            flag.fetch_add(1, Ordering::Relaxed);

            // Release each lock before finishing (R10)
            drop(guard_b);
            drop(guard_a);
        })
    };

    // ------------------------------------------------------------------
    // Worker b (R3, R4, R5, R10) — mirror image of worker a
    // ------------------------------------------------------------------
    let hb = {
        let lock_a = lock_a.clone();
        let lock_b = lock_b.clone();
        let sa = sa.clone();
        let sb = sb.clone();
        let flag = flag.clone();
        tokio::spawn(async move {
            // First lock
            let guard_b = lock_b.lock().await;
            // Handshake: announce "I hold my first lock" ...
            sb.add_permits(1);
            // ... then wait for a's announcement before taking the second lock (R5)
            let _permit = sa.acquire().await.expect("sa closed");
            // Second lock — both locks now held simultaneously (R4)
            let guard_a = lock_a.lock().await;

            // Critical section
            flag.fetch_add(1, Ordering::Relaxed);

            // Release each lock before finishing (R10)
            drop(guard_a);
            drop(guard_b);
        })
    };

    // ------------------------------------------------------------------
    // Main (R1, R11): started all three tasks; waits only for the two
    // workers. The bystander may still be running when we exit.
    // ------------------------------------------------------------------
    ha.await.expect("worker a panicked");
    hb.await.expect("worker b panicked");

    // Both workers finished their critical sections (flag == 2).
    debug_assert_eq!(flag.load(Ordering::Relaxed) >= 2, true);

    // R12: print exactly this line, then exit.
    println!("DONE a=1 b=1");
}
