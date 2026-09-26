mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

use concir_sync::Semaphore;

struct State {
    waiters: usize,
    arrived: usize,
    go: bool,
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", State {
        waiters: 2,
        arrived: 0,
        go: false,
    }));
    let cv = Arc::new(Condvar::new_named("cv_condvar0"));
    let g12 = Semaphore::new_named("g12_semaphore0", 2);
    let gN = Semaphore::new_named("gN_semaphore0", 2);

    let m1 = Arc::clone(&m);
    let cv1 = Arc::clone(&cv);
    let g12_1 = Arc::clone(&g12);
    let gN_1 = Arc::clone(&gN);
    let w1 = cir_trace::spawn("w1", move || {
        // Take the initial permits on this thread so the notifier cannot
        // observe them before w1 is the owner. Release the readiness permit
        // only while holding m, immediately before waiting.
        let ready = g12_1.acquire();
        let done = gN_1.acquire();
        {
            let mut guard = m1.lock().unwrap();
            guard.arrived += 1;
            cv1.notify_all();
            ready.release();
            while !guard.go {
                guard = cv1.wait(guard).unwrap();
            }
            guard.waiters -= 1;
        }
        done.release();
    });

    let m2 = Arc::clone(&m);
    let cv2 = Arc::clone(&cv);
    let g12_2 = Arc::clone(&g12);
    let gN_2 = Arc::clone(&gN);
    let w2 = cir_trace::spawn("w2", move || {
        let ready = g12_2.acquire();
        let done = gN_2.acquire();
        {
            let mut guard = m2.lock().unwrap();
            guard.arrived += 1;
            cv2.notify_all();
            ready.release();
            while !guard.go {
                guard = cv2.wait(guard).unwrap();
            }
            guard.waiters -= 1;
        }
        done.release();
    });

    let m_n = Arc::clone(&m);
    let cv_n = Arc::clone(&cv);
    let g12_n = Arc::clone(&g12);
    let gN_n = Arc::clone(&gN);
    let notifier = cir_trace::spawn("notifier", move || {
        {
            let mut guard = m_n.lock().unwrap();
            while guard.arrived < 2 {
                guard = cv_n.wait(guard).unwrap();
            }
        }
        // Both waiters own the initial g12 permits until they are about to
        // wait, so these acquires complete only after both have checked in.
        let permit_a = g12_n.acquire();
        let permit_b = g12_n.acquire();
        drop(permit_a);
        drop(permit_b);
        {
            let mut guard = m_n.lock().unwrap();
            guard.go = true;
            cv_n.notify_all();
        }
        let finished_a = gN_n.acquire();
        let finished_b = gN_n.acquire();
        drop(finished_a);
        drop(finished_b);
    });

    w1.join().unwrap();
    w2.join().unwrap();
    notifier.join().unwrap();

    let waiters = m.lock().unwrap().waiters;
    println!("DONE waiters={waiters}");
 cir_trace::finish();}
