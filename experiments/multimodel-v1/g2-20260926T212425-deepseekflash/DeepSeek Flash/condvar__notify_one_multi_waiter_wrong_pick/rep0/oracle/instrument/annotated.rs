mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;
use concir_sync::Semaphore;

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", ()));
    let cv = Arc::new(Condvar::new_named("cv_condvar0"));

    // g12: counts waiters that are ready (notifier acquires 2)
    let g12 = Semaphore::new_named("g12_semaphore0", 0);

    // gN: notifier releases 2 permits; each waiter acquires 1
    let gN = Semaphore::new_named("gN_semaphore0", 0);

    // Shared flag: whether waiters may proceed
    let proceed = Arc::new(Mutex::new_named("proceed_mutex0", false));

    let m1 = Arc::clone(&m);
    let cv1 = Arc::clone(&cv);
    let g12_w1 = Arc::clone(&g12);
    let gN_w1 = Arc::clone(&gN);
    let p1 = Arc::clone(&proceed);

    let w1 = cir_trace::spawn("w1", move || {
        // Signal readiness.
        g12_w1.acquire().release();

        // Wait on cv while holding the lock.
        let mut guard = m1.lock().unwrap();
        while !*p1.lock().unwrap() {
            guard = cv1.wait(guard).unwrap();
        }
        drop(guard);

        // Consume our signal permit.
        gN_w1.acquire().release();
    });

    let m2 = Arc::clone(&m);
    let cv2 = Arc::clone(&cv);
    let g12_w2 = Arc::clone(&g12);
    let gN_w2 = Arc::clone(&gN);
    let p2 = Arc::clone(&proceed);

    let w2 = cir_trace::spawn("w2", move || {
        g12_w2.acquire().release();

        let mut guard = m2.lock().unwrap();
        while !*p2.lock().unwrap() {
            guard = cv2.wait(guard).unwrap();
        }
        drop(guard);

        gN_w2.acquire().release();
    });

    let m3 = Arc::clone(&m);
    let cv3 = Arc::clone(&cv);
    let g12_n = Arc::clone(&g12);
    let gN_n = Arc::clone(&gN);
    let p3 = Arc::clone(&proceed);

    let notifier = cir_trace::spawn("notifier", move || {
        // Wait until both waiters are ready.
        let pa = g12_n.acquire();
        let pb = g12_n.acquire();
        pa.release();
        pb.release();

        // Take the lock before waking waiters.
        let guard = m3.lock().unwrap();

        // Set the flag so waiters proceed.
        *p3.lock().unwrap() = true;

        // Release two permits on gN so both waiters can proceed.
        gN_n.acquire().release();
        gN_n.acquire().release();

        // Wake all waiters.
        cv3.notify_all();

        // Release the lock afterwards.
        drop(guard);
    });

    w1.join().unwrap();
    w2.join().unwrap();
    notifier.join().unwrap();

    println!("DONE waiters=0");
 cir_trace::finish();}
