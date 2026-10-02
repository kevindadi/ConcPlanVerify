use concir_sync::{Permit, Semaphore};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

// w1: lock m, signal g12, wait on cv while holding m, unlock m, signal gN.
fn w1(m: Arc<Mutex<()>>, cv: Arc<Condvar>, tok12: Permit, tok_n: Permit) {
    let guard = m.lock().unwrap();
    tok12.release();
    let guard = cv.wait(guard).unwrap();
    drop(guard);
    tok_n.release();
}

// w2: identical role to w1.
fn w2(m: Arc<Mutex<()>>, cv: Arc<Condvar>, tok12: Permit, tok_n: Permit) {
    let guard = m.lock().unwrap();
    tok12.release();
    let guard = cv.wait(guard).unwrap();
    drop(guard);
    tok_n.release();
}

// notifier: wait until both waiters announced themselves on g12, then take the
// lock, wake everyone on cv, release the lock, and wait for both waiters to
// finish (gN twice).
fn notifier(m: Arc<Mutex<()>>, cv: Arc<Condvar>, g12: Arc<Semaphore>, gN: Arc<Semaphore>) {
    let p1 = g12.acquire();
    let p2 = g12.acquire();
    let guard = m.lock().unwrap();
    cv.notify_all();
    drop(guard);
    let p3 = gN.acquire();
    let p4 = gN.acquire();
    // The design consumes these permits and never releases them, so they must
    // not be handed back (dropping a permit would release it again).
    std::mem::forget(p1);
    std::mem::forget(p2);
    std::mem::forget(p3);
    std::mem::forget(p4);
}

fn main() {
    let m = Arc::new(Mutex::new(()));
    let cv = Arc::new(Condvar::new());

    // In the design g12 and gN are counting semaphores whose counts start at 0.
    // The counters are held by the release side, i.e. the waiters: main takes
    // the initial permits out of both semaphores before any thread runs, so the
    // counters really are 0 once the scope is entered and the waiters later
    // release exactly those permits.
    let g12 = Semaphore::new(2);
    let gN = Semaphore::new(2);
    let tok12_w1 = g12.acquire();
    let tok12_w2 = g12.acquire();
    let tok_n_w1 = gN.acquire();
    let tok_n_w2 = gN.acquire();

    let m1 = Arc::clone(&m);
    let cv1 = Arc::clone(&cv);
    let t1 = thread::spawn(move || w1(m1, cv1, tok12_w1, tok_n_w1));

    let m2 = Arc::clone(&m);
    let cv2 = Arc::clone(&cv);
    let t2 = thread::spawn(move || w2(m2, cv2, tok12_w2, tok_n_w2));

    let m3 = Arc::clone(&m);
    let cv3 = Arc::clone(&cv);
    let g12_n = Arc::clone(&g12);
    let gN_n = Arc::clone(&gN);
    let t3 = thread::spawn(move || notifier(m3, cv3, g12_n, gN_n));

    t1.join().unwrap();
    t2.join().unwrap();
    t3.join().unwrap();

    println!("DONE waiters=0");
}
