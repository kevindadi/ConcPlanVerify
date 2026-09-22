use std::sync::{Arc, Condvar, Mutex};
use std::thread;

struct State {
    // number of waiters that have arrived and are ready to wait
    ready: usize,
    // number of waiters still blocked
    waiting: usize,
    // whether the notifier has signalled
    notified: bool,
}

fn main() {
    let m = Arc::new(Mutex::new(State {
        ready: 0,
        waiting: 0,
        notified: false,
    }));
    let cv = Arc::new(Condvar::new());

    // g12: semaphore counting ready waiters (both must be ready)
    // gN: semaphore counting notifier readiness
    let g12 = Arc::new((Mutex::new(0usize), Condvar::new()));
    let gN = Arc::new((Mutex::new(0usize), Condvar::new()));

    let m1 = Arc::clone(&m);
    let cv1 = Arc::clone(&cv);
    let g12_1 = Arc::clone(&g12);
    let gN_1 = Arc::clone(&gN);

    let w1 = thread::spawn(move || {
        // Signal readiness (V on g12)
        {
            let (lk, c) = &*g12_1;
            let mut v = lk.lock().unwrap();
            *v += 1;
            c.notify_one();
        }
        // Wait until notifier is ready (P on gN)
        {
            let (lk, c) = &*gN_1;
            let mut v = lk.lock().unwrap();
            while *v == 0 {
                v = c.wait(v).unwrap();
            }
            *v -= 1;
        }
        // Now wait on the condition variable holding the lock
        let mut st = m1.lock().unwrap();
        st.waiting += 1;
        while !st.notified {
            st = cv1.wait(st).unwrap();
        }
        st.waiting -= 1;
    });

    let m2 = Arc::clone(&m);
    let cv2 = Arc::clone(&cv);
    let g12_2 = Arc::clone(&g12);
    let gN_2 = Arc::clone(&gN);

    let w2 = thread::spawn(move || {
        {
            let (lk, c) = &*g12_2;
            let mut v = lk.lock().unwrap();
            *v += 1;
            c.notify_one();
        }
        {
            let (lk, c) = &*gN_2;
            let mut v = lk.lock().unwrap();
            while *v == 0 {
                v = c.wait(v).unwrap();
            }
            *v -= 1;
        }
        let mut st = m2.lock().unwrap();
        st.waiting += 1;
        while !st.notified {
            st = cv2.wait(st).unwrap();
        }
        st.waiting -= 1;
    });

    let m3 = Arc::clone(&m);
    let cv3 = Arc::clone(&cv);
    let g12_3 = Arc::clone(&g12);
    let gN_3 = Arc::clone(&gN);

    let notifier = thread::spawn(move || {
        // Wait until both waiters are ready (P twice on g12)
        {
            let (lk, c) = &*g12_3;
            let mut v = lk.lock().unwrap();
            while *v < 2 {
                v = c.wait(v).unwrap();
            }
            *v -= 2;
        }
        // Signal notifier readiness to both waiters (V twice on gN)
        {
            let (lk, c) = &*gN_3;
            let mut v = lk.lock().unwrap();
            *v += 2;
            c.notify_all();
        }
        // Take the lock, wake all waiters, release the lock
        let mut st = m3.lock().unwrap();
        st.notified = true;
        cv3.notify_all();
        drop(st);
    });

    w1.join().unwrap();
    w2.join().unwrap();
    notifier.join().unwrap();

    let st = m.lock().unwrap();
    println!("DONE waiters={}", st.waiting);
}
