use std::sync::{Arc, Condvar, Mutex};
use std::thread;

use concir_sync::Semaphore;

struct State {
    waiters: usize,
    go: bool,
    ready_holds: usize,
}

fn main() {
    let m = Arc::new(Mutex::new(State {
        waiters: 2,
        go: false,
        ready_holds: 0,
    }));
    let cv = Arc::new(Condvar::new());
    let g12 = Semaphore::new(2);
    let gN = Semaphore::new(2);

    let m1 = Arc::clone(&m);
    let cv1 = Arc::clone(&cv);
    let g12_1 = Arc::clone(&g12);
    let gN_1 = Arc::clone(&gN);
    let w1 = thread::spawn(move || {
        let ready = g12_1.acquire();
        let done = gN_1.acquire();
        let mut guard = m1.lock().unwrap();
        guard.ready_holds += 1;
        cv1.notify_all();
        ready.release();
        while !guard.go {
            guard = cv1.wait(guard).unwrap();
        }
        guard.waiters -= 1;
        drop(guard);
        done.release();
    });

    let m2 = Arc::clone(&m);
    let cv2 = Arc::clone(&cv);
    let g12_2 = Arc::clone(&g12);
    let gN_2 = Arc::clone(&gN);
    let w2 = thread::spawn(move || {
        let ready = g12_2.acquire();
        let done = gN_2.acquire();
        let mut guard = m2.lock().unwrap();
        guard.ready_holds += 1;
        cv2.notify_all();
        ready.release();
        while !guard.go {
            guard = cv2.wait(guard).unwrap();
        }
        guard.waiters -= 1;
        drop(guard);
        done.release();
    });

    let m_n = Arc::clone(&m);
    let cv_n = Arc::clone(&cv);
    let g12_n = Arc::clone(&g12);
    let gN_n = Arc::clone(&gN);
    let notifier = thread::spawn(move || {
        {
            let mut guard = m_n.lock().unwrap();
            while guard.ready_holds < 2 {
                guard = cv_n.wait(guard).unwrap();
            }
        }
        let permit_a = g12_n.acquire();
        let permit_b = g12_n.acquire();
        {
            let mut guard = m_n.lock().unwrap();
            guard.go = true;
            cv_n.notify_all();
        }
        drop(permit_a);
        drop(permit_b);
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
}
