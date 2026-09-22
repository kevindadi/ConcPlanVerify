use std::sync::{Arc, Condvar, Mutex};
use std::thread;

struct State {
    // permit counter: number of waiters that have arrived and are ready to wait
    ready: usize,
    // number of waiters still blocked
    waiters: usize,
}

fn main() {
    let m = Arc::new((Mutex::new(State { ready: 0, waiters: 2 }), Condvar::new()));
    let g12 = Arc::new((Mutex::new(0usize), Condvar::new()));
    let gN = Arc::new((Mutex::new(0usize), Condvar::new()));

    let m1 = Arc::clone(&m);
    let g12_1 = Arc::clone(&g12);
    let gN_1 = Arc::clone(&gN);
    let w1 = thread::spawn(move || {
        // signal readiness
        {
            let (lock, cvar) = &*g12_1;
            let mut n = lock.lock().unwrap();
            *n += 1;
            cvar.notify_all();
        }
        // wait until notifier says both are ready
        {
            let (lock, cvar) = &*gN_1;
            let mut n = lock.lock().unwrap();
            while *n == 0 {
                n = cvar.wait(n).unwrap();
            }
        }
        // now block on cv
        let (lock, cvar) = &*m1;
        let mut state = lock.lock().unwrap();
        while state.waiters > 0 {
            state = cvar.wait(state).unwrap();
        }
    });

    let m2 = Arc::clone(&m);
    let g12_2 = Arc::clone(&g12);
    let gN_2 = Arc::clone(&gN);
    let w2 = thread::spawn(move || {
        {
            let (lock, cvar) = &*g12_2;
            let mut n = lock.lock().unwrap();
            *n += 1;
            cvar.notify_all();
        }
        {
            let (lock, cvar) = &*gN_2;
            let mut n = lock.lock().unwrap();
            while *n == 0 {
                n = cvar.wait(n).unwrap();
            }
        }
        let (lock, cvar) = &*m2;
        let mut state = lock.lock().unwrap();
        while state.waiters > 0 {
            state = cvar.wait(state).unwrap();
        }
    });

    let m3 = Arc::clone(&m);
    let g12_3 = Arc::clone(&g12);
    let gN_3 = Arc::clone(&gN);
    let notifier = thread::spawn(move || {
        // wait until both waiters are ready
        {
            let (lock, cvar) = &*g12_3;
            let mut n = lock.lock().unwrap();
            while *n < 2 {
                n = cvar.wait(n).unwrap();
            }
        }
        // release waiters to block on cv
        {
            let (lock, cvar) = &*gN_3;
            let mut n = lock.lock().unwrap();
            *n = 1;
            cvar.notify_all();
        }
        // take lock, wake all waiters, release lock
        let (lock, cvar) = &*m3;
        let mut state = lock.lock().unwrap();
        state.waiters = 0;
        cvar.notify_all();
        drop(state);
    });

    w1.join().unwrap();
    w2.join().unwrap();
    notifier.join().unwrap();

    println!("DONE waiters=0");
}
