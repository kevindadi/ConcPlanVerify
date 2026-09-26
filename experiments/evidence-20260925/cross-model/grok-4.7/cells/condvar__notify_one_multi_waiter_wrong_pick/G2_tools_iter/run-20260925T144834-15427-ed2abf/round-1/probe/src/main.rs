use std::sync::{Arc, Condvar, Mutex};
use std::thread;

use concir_sync::Semaphore;

struct State {
    waiters: usize,
    go: bool,
}

fn main() {
    let m = Arc::new(Mutex::new(State {
        waiters: 2,
        go: false,
    }));
    let cv = Arc::new(Condvar::new());
    let g12 = Semaphore::new(2);
    let gN = Semaphore::new(2);

    let ready_w1 = g12.acquire();
    let ready_w2 = g12.acquire();
    let done_w1 = gN.acquire();
    let done_w2 = gN.acquire();

    let m1 = Arc::clone(&m);
    let cv1 = Arc::clone(&cv);
    let w1 = thread::spawn(move || {
        let mut guard = m1.lock().unwrap();
        ready_w1.release();
        while !guard.go {
            guard = cv1.wait(guard).unwrap();
        }
        guard.waiters -= 1;
        drop(guard);
        done_w1.release();
    });

    let m2 = Arc::clone(&m);
    let cv2 = Arc::clone(&cv);
    let w2 = thread::spawn(move || {
        let mut guard = m2.lock().unwrap();
        ready_w2.release();
        while !guard.go {
            guard = cv2.wait(guard).unwrap();
        }
        guard.waiters -= 1;
        drop(guard);
        done_w2.release();
    });

    let m_n = Arc::clone(&m);
    let cv_n = Arc::clone(&cv);
    let g12_n = Arc::clone(&g12);
    let gN_n = Arc::clone(&gN);
    let notifier = thread::spawn(move || {
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
