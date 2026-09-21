use std::sync::{Arc, Mutex, MutexGuard, TryLockError};
use std::thread;

/// Acquire two mutexes while guaranteeing that a deadlock is impossible.
///
/// The first mutex is taken with a blocking `lock`. The second mutex is
/// taken with `try_lock`: if it is currently held by another thread, we
/// release the first mutex and retry. Because a thread never *blocks*
/// while already holding a mutex, no circular chain of blocking waits can
/// ever form, so no interleaving can deadlock or livelock on the locks.
///
/// On top of that, every call site passes its two mutexes in the global
/// order A < B < C, so even the transient "holds first, wants second"
/// states can never cycle: the thread whose second mutex is last in the
/// global order always finds it free.
fn lock_pair(
    first: &Mutex<()>,
    second: &Mutex<()>,
) -> (MutexGuard<'_, ()>, MutexGuard<'_, ()>) {
    loop {
        let first_guard = first.lock().unwrap();
        match second.try_lock() {
            Ok(second_guard) => return (first_guard, second_guard),
            Err(TryLockError::WouldBlock) => {
                // Second mutex is contended; release the first and retry.
                drop(first_guard);
            }
            Err(TryLockError::Poisoned(p)) => {
                // Propagate poisoning rather than ignoring it: recover the
                // guard from the error and proceed with both locks held.
                // (The guard inside a PoisonError is not `Send`, so it
                // cannot be re-panicked via `panic_any`; unwrapping the
                // error is the correct way to surface the poisoned lock.)
                let second_guard = p.into_inner();
                return (first_guard, second_guard);
            }
        }
    }
}

fn main() {
    let mtx_a = Arc::new(Mutex::new(()));
    let mtx_b = Arc::new(Mutex::new(()));
    let mtx_c = Arc::new(Mutex::new(()));

    // Global lock order: A < B < C. Every worker acquires its two mutexes
    // in this order, and the second acquisition is a non-blocking
    // try_lock with release-and-retry, so the circular wait (and thus
    // deadlock) is impossible under every interleaving.

    // Worker 1: A and B (A first, then B).
    let (a1, b1) = (Arc::clone(&mtx_a), Arc::clone(&mtx_b));
    let w1 = thread::spawn(move || {
        let (ga, gb) = lock_pair(&a1, &b1);
        drop(gb);
        drop(ga);
    });

    // Worker 2: B and C (B first, then C).
    let (b2, c2) = (Arc::clone(&mtx_b), Arc::clone(&mtx_c));
    let w2 = thread::spawn(move || {
        let (gb, gc) = lock_pair(&b2, &c2);
        drop(gc);
        drop(gb);
    });

    // Worker 3: C and A; it must acquire them in global order: A first, then C.
    let (c3, a3) = (Arc::clone(&mtx_c), Arc::clone(&mtx_a));
    let w3 = thread::spawn(move || {
        let (ga, gc) = lock_pair(&a3, &c3);
        drop(gc);
        drop(ga);
    });

    w1.join().unwrap();
    w2.join().unwrap();
    w3.join().unwrap();

    println!("DONE done=1");
}
