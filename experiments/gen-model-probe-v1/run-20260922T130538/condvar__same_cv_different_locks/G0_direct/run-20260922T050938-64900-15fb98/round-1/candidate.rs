use std::sync::{Arc, Condvar, Mutex};
use std::thread;

struct Shared {
    // Per-waiter predicates, each guarded by its own distinct lock (R2).
    flag1: Mutex<bool>,
    flag2: Mutex<bool>,
    // The single condition variable both waiters block on (R3).
    wake: Condvar,
    // Announcement counter so the notifier knows how many waits to expect (R5).
    announced: Mutex<usize>,
    announced_cv: Condvar,
}

fn waiter(shared: Arc<Shared>, first: bool) {
    // Each waiter guards its work with its own lock (R2).
    let flag_mutex = if first { &shared.flag1 } else { &shared.flag2 };
    let mut flag = flag_mutex.lock().unwrap();

    // Announce that this waiter is about to wait, while still holding its own
    // lock, so an announcement guarantees the waiter will block (or is blocked)
    // before its lock can be taken by the notifier (R5).
    {
        let mut n = shared.announced.lock().unwrap();
        *n += 1;
        shared.announced_cv.notify_all();
    }

    // Block on the shared condition variable while holding our own lock (R3, R4).
    // The predicate loop makes wakeup independent of any race between waiters (R8).
    while !*flag {
        flag = shared.wake.wait(flag).unwrap();
    }
}

fn notifier(shared: Arc<Shared>) {
    // Wait until both waiters have announced themselves (R6).
    {
        let mut n = shared.announced.lock().unwrap();
        while *n < 2 {
            n = shared.announced_cv.wait(n).unwrap();
        }
    }

    // Wake waiter 1: hold the lock that waiter needs in order to wake and
    // finish, set its predicate, then wake everyone on the shared condvar (R7).
    // Holding the lock guarantees the waiter is blocked inside `wait`, so no
    // wakeup can be lost (R8).
    {
        let mut flag = shared.flag1.lock().unwrap();
        *flag = true;
        shared.wake.notify_all();
    }

    // Wake waiter 2 the same way, holding its lock (R7, R8).
    {
        let mut flag = shared.flag2.lock().unwrap();
        *flag = true;
        shared.wake.notify_all();
    }
}

fn main() {
    let shared = Arc::new(Shared {
        flag1: Mutex::new(false),
        flag2: Mutex::new(false),
        wake: Condvar::new(),
        announced: Mutex::new(0),
        announced_cv: Condvar::new(),
    });

    // Start two waiters and one notifier at the same time (R1).
    let s1 = Arc::clone(&shared);
    let s2 = Arc::clone(&shared);
    let s3 = Arc::clone(&shared);

    let w1 = thread::spawn(move || waiter(s1, true));
    let w2 = thread::spawn(move || waiter(s2, false));
    let n = thread::spawn(move || notifier(s3));

    // Every role finishes in every interleaving (R9).
    w1.join().unwrap();
    w2.join().unwrap();
    n.join().unwrap();

    println!("DONE done=1");
}
