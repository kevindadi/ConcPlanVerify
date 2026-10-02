use concir_sync::{Permit, Semaphore};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

// w1: lock m, announce readiness on g12, wait on cv (releasing m while
// waiting), re-acquire m, unlock it, then signal completion on gN.
fn w1(m: Arc<Mutex<()>>, cv: Arc<Condvar>, t12: Permit, tn: Permit) {
    let guard = m.lock().unwrap();
    t12.release();
    let guard = cv.wait(guard).unwrap();
    drop(guard);
    tn.release();
}

// w2: the same role as w1.
fn w2(m: Arc<Mutex<()>>, cv: Arc<Condvar>, t12: Permit, tn: Permit) {
    let guard = m.lock().unwrap();
    t12.release();
    let guard = cv.wait(guard).unwrap();
    drop(guard);
    tn.release();
}

// notifier: wait until both waiters announced themselves on g12, take the
// lock, wake every waiter on cv, release the lock, then wait until both
// waiters finished (gN twice).
fn notifier(m: Arc<Mutex<()>>, cv: Arc<Condvar>, g12: Arc<Semaphore>, gN: Arc<Semaphore>) {
    let p1 = g12.acquire();
    let p2 = g12.acquire();
    let guard = m.lock().unwrap();
    cv.notify_all();
    drop(guard);
    let p3 = gN.acquire();
    let p4 = gN.acquire();
    // The design consumes these permits and never releases them, so prevent
    // their Drop from handing the permits back to the semaphores.
    std::mem::forget(p1);
    std::mem::forget(p2);
    std::mem::forget(p3);
    std::mem::forget(p4);
}

fn main() {
    let m = Arc::new(Mutex::new(()));
    let cv = Arc::new(Condvar::new());

    // In the design g12 and gN are counting semaphores whose counts start at 0.
    // Create each with the two permits that the waiters will later release and
    // take those permits out here, so the counters really are 0 once the scope
    // is entered. The semaphores get a 'static home so the permits can travel
    // to the spawned threads.
    let g12: &'static Arc<Semaphore> = Box::leak(Box::new(Semaphore::new(2)));
    let gN: &'static Arc<Semaphore> = Box::leak(Box::new(Semaphore::new(2)));

    let t12_w1 = g12.acquire();
    let t12_w2 = g12.acquire();
    let tn_w1 = gN.acquire();
    let tn_w2 = gN.acquire();

    let m1 = Arc::clone(&m);
    let cv1 = Arc::clone(&cv);
    let t1 = thread::spawn(move || w1(m1, cv1, t12_w1, tn_w1));

    let m2 = Arc::clone(&m);
    let cv2 = Arc::clone(&cv);
    let t2 = thread::spawn(move || w2(m2, cv2, t12_w2, tn_w2));

    let m3 = Arc::clone(&m);
    let cv3 = Arc::clone(&cv);
    let g12n = Arc::clone(g12);
    let gNn = Arc::clone(gN);
    let t3 = thread::spawn(move || notifier(m3, cv3, g12n, gNn));

    t1.join().unwrap();
    t2.join().unwrap();
    t3.join().unwrap();

    println!("DONE waiters=0");
}
