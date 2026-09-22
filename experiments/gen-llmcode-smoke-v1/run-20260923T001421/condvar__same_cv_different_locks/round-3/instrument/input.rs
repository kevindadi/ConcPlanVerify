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
    // s1: lock m1
    let mut a_guard = shared.m1.lock().unwrap();
    // s2: read announced
    let mut a = *shared.announced.lock().unwrap();
    // s3: a2 = a + 1
    let a2 = a + 1;
    // s4: write announced
    *shared.announced.lock().unwrap() = a2;
    a = a2;
    // s5: branch announced == 2
    loop {
        if *shared.announced.lock().unwrap() == 2 {
            break;
        }
        // s6: condvar_wait on cv with m1
        a_guard = shared.cv.wait(a_guard).unwrap();
    }
    // s8: unlock m1
    drop(a_guard);
    // s9: lock m2
    let mut d_guard = shared.m2.lock().unwrap();
    // s10: read done
    let d = *shared.done.lock().unwrap();
    // s11: d2 = d + 1
    let d2 = d + 1;
    // s12: write done
    *shared.done.lock().unwrap() = d2;
    // s13: unlock m2
    drop(d_guard);
    // s14: return
}

fn waiter2(shared: Arc<Shared>) {
    // s1: lock m2
    let m2_guard = shared.m2.lock().unwrap();
    // s2: lock m1
    let mut a_guard = shared.m1.lock().unwrap();
    // s3: read announced
    let mut a = *shared.announced.lock().unwrap();
    // s4: a2 = a + 1
    let a2 = a + 1;
    // s5: write announced
    *shared.announced.lock().unwrap() = a2;
    a = a2;
    // s6: branch announced == 2
    loop {
        if *shared.announced.lock().unwrap() == 2 {
            break;
        }
        // s7: condvar_wait on cv with m1
        a_guard = shared.cv.wait(a_guard).unwrap();
    }
    // s9: unlock m1
    drop(a_guard);
    // s10: unlock m2
    drop(m2_guard);
    // s11: lock m2
    let d_guard = shared.m2.lock().unwrap();
    // s12: read done
    let d = *shared.done.lock().unwrap();
    // s13: d2 = d + 1
    let d2 = d + 1;
    // s14: write done
    *shared.done.lock().unwrap() = d2;
    // s15: unlock m2
    drop(d_guard);
    // s16: return
}

fn notifier(shared: Arc<Shared>) {
    // s1: lock m1
    let a_guard = shared.m1.lock().unwrap();
    // s2: read announced
    let mut a = *shared.announced.lock().unwrap();
    // s3: branch a == 2
    while a != 2 {
        // s4: goto s2
        a = *shared.announced.lock().unwrap();
    }
    // s5: unlock m1
    drop(a_guard);
    // s6: lock m2
    let m2_guard = shared.m2.lock().unwrap();
    // s7: condvar_notify_all on cv
    shared.cv.notify_all();
    // s8: unlock m2
    drop(m2_guard);
    // s9: return
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

    let done = *shared.done.lock().unwrap();
    println!("DONE done={}", done);
}
