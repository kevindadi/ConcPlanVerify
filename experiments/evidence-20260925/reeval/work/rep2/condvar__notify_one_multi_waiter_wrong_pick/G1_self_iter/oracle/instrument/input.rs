use std::sync::{Arc, Condvar, Mutex};
use std::thread;

struct Shared {
    m: Mutex<State>,
    cv: Condvar,
}

struct State {
    // permit counter: number of waiters that have signaled readiness
    ready: u32,
    // number of waiters still blocked
    blocked: u32,
    // whether notifier has fired
    notified: bool,
}

fn main() {
    let shared = Arc::new(Shared {
        m: Mutex::new(State {
            ready: 0,
            blocked: 0,
            notified: false,
        }),
        cv: Condvar::new(),
    });

    // g12: semaphore-like counter for waiters ready (implemented via mutex+cv)
    // gN: semaphore-like counter for notifier readiness
    // We'll use the same shared state to track these.

    let s1 = Arc::clone(&shared);
    let s2 = Arc::clone(&shared);
    let sn = Arc::clone(&shared);

    let w1 = thread::spawn(move || {
        let mut st = s1.m.lock().unwrap();
        st.ready += 1;
        st.blocked += 1;
        s1.cv.notify_all();
        // wait until notified
        while !st.notified {
            st = s1.cv.wait(st).unwrap();
        }
        st.blocked -= 1;
        // done
    });

    let w2 = thread::spawn(move || {
        let mut st = s2.m.lock().unwrap();
        st.ready += 1;
        st.blocked += 1;
        s2.cv.notify_all();
        while !st.notified {
            st = s2.cv.wait(st).unwrap();
        }
        st.blocked -= 1;
    });

    let notifier = thread::spawn(move || {
        let mut st = sn.m.lock().unwrap();
        // wait until both waiters are ready
        while st.ready < 2 {
            st = sn.cv.wait(st).unwrap();
        }
        // wake all waiters
        st.notified = true;
        sn.cv.notify_all();
        // release lock
        drop(st);
    });

    w1.join().unwrap();
    w2.join().unwrap();
    notifier.join().unwrap();

    let st = shared.m.lock().unwrap();
    println!("DONE waiters={}", st.blocked);
}
