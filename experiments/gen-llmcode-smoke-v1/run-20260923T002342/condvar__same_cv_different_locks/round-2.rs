use std::sync::{Arc, Condvar, Mutex};
use std::thread;

struct Shared {
    cv: Condvar,
    m1: Mutex<i32>,
    m2: Mutex<i32>,
    announced: Mutex<i32>,
    done: Mutex<i32>,
}

fn waiter1(shared: Arc<Shared>) {
    // s1: mutex_lock m1
    let mut m1_guard = shared.m1.lock().unwrap();
    // s2-s4: read announced, a2 = a + 1, write announced
    {
        let mut a = shared.announced.lock().unwrap();
        let a2 = *a + 1;
        *a = a2;
    }
    // s5: branch announced == 2
    loop {
        let announced_val = *shared.announced.lock().unwrap();
        if announced_val == 2 {
            break;
        }
        // s6: condvar_wait cv on m1
        m1_guard = shared.cv.wait(m1_guard).unwrap();
    }
    // s8: mutex_unlock m1
    drop(m1_guard);
    // s9: mutex_lock m2
    let mut m2_guard = shared.m2.lock().unwrap();
    // s10-s12: read done, d2 = d + 1, write done
    {
        let mut d = shared.done.lock().unwrap();
        let d2 = *d + 1;
        *d = d2;
    }
    // s13: mutex_unlock m2
    drop(m2_guard);
}

fn waiter2(shared: Arc<Shared>) {
    // s1: mutex_lock m2
    let mut m2_guard = shared.m2.lock().unwrap();
    // s2: mutex_lock m1
    let mut m1_guard = shared.m1.lock().unwrap();
    // s3-s5: read announced, a2 = a + 1, write announced
    {
        let mut a = shared.announced.lock().unwrap();
        let a2 = *a + 1;
        *a = a2;
    }
    // s6: branch announced == 2
    loop {
        let announced_val = *shared.announced.lock().unwrap();
        if announced_val == 2 {
            break;
        }
        // s7: condvar_wait cv on m1
        m1_guard = shared.cv.wait(m1_guard).unwrap();
    }
    // s9: mutex_unlock m1
    drop(m1_guard);
    // s10: mutex_unlock m2
    drop(m2_guard);
    // s11: mutex_lock m2
    let mut m2_guard = shared.m2.lock().unwrap();
    // s12-s14: read done, d2 = d + 1, write done
    {
        let mut d = shared.done.lock().unwrap();
        let d2 = *d + 1;
        *d = d2;
    }
    // s15: mutex_unlock m2
    drop(m2_guard);
}

fn notifier(shared: Arc<Shared>) {
    // s1: mutex_lock m1
    let mut m1_guard = shared.m1.lock().unwrap();
    // s2-s4: read announced, loop until a == 2
    loop {
        let a = *shared.announced.lock().unwrap();
        if a == 2 {
            break;
        }
    }
    // s5: mutex_unlock m1
    drop(m1_guard);
    // s6: mutex_lock m2
    let m2_guard = shared.m2.lock().unwrap();
    // s7: condvar_notify_all cv
    shared.cv.notify_all();
    // s8: mutex_unlock m2
    drop(m2_guard);
}

fn main() {
    let shared = Arc::new(Shared {
        cv: Condvar::new(),
        m1: Mutex::new(0),
        m2: Mutex::new(0),
        announced: Mutex::new(0),
        done: Mutex::new(0),
    });

    let s1 = Arc::clone(&shared);
    let s2 = Arc::clone(&shared);
    let s3 = Arc::clone(&shared);

    let t1 = thread::spawn(move || waiter1(s1));
    let t2 = thread::spawn(move || waiter2(s2));
    let t3 = thread::spawn(move || notifier(s3));

    t1.join().unwrap();
    t2.join().unwrap();
    t3.join().unwrap();

    let done_val = *shared.done.lock().unwrap();
    println!("DONE done={}", done_val);
}
