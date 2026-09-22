mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct Shared {
    cv: Condvar,
    m1: Mutex<i32>,
    m2: Mutex<i32>,
    announced: Mutex<i32>,
    done: Mutex<i32>,
}

fn waiter1(shared: Arc<Shared>) {
    {
        let mut announced = shared.m1.lock().unwrap();
        let a = *announced;
        let a2 = a + 1;
        *announced = a2;
        while *announced != 2 {
            announced = shared.cv.wait(announced).unwrap();
        }
    }
    {
        let mut done = shared.m2.lock().unwrap();
        let d = *done;
        let d2 = d + 1;
        *done = d2;
    }
}

fn waiter2(shared: Arc<Shared>) {
    let mut m2_guard = shared.m2.lock().unwrap();
    {
        let mut announced = shared.m1.lock().unwrap();
        let a = *announced;
        let a2 = a + 1;
        *announced = a2;
        while *announced != 2 {
            announced = shared.cv.wait(announced).unwrap();
        }
    }
    drop(m2_guard);
    {
        let mut done = shared.m2.lock().unwrap();
        let d = *done;
        let d2 = d + 1;
        *done = d2;
    }
}

fn notifier(shared: Arc<Shared>) {
    {
        let mut announced = shared.m1.lock().unwrap();
        while *announced != 2 {
            announced = shared.cv.wait(announced).unwrap();
        }
    }
    {
        let _m2_guard = shared.m2.lock().unwrap();
        shared.cv.notify_all();
    }
}

fn main() { cir_trace::init();
    let shared = Arc::new(Shared {
        cv: Condvar::new_named("shared_condvar0"),
        m1: Mutex::new_named("shared_mutex0", 0),
        m2: Mutex::new_named("shared_mutex1", 0),
        announced: Mutex::new_named("shared_mutex2", 0),
        done: Mutex::new_named("shared_mutex3", 0),
    });

    let s1 = Arc::clone(&shared);
    let s2 = Arc::clone(&shared);
    let s3 = Arc::clone(&shared);

    let t1 = cir_trace::spawn("t1", move || waiter1(s1));
    let t2 = cir_trace::spawn("t2", move || waiter2(s2));
    let t3 = cir_trace::spawn("t3", move || notifier(s3));

    t1.join().unwrap();
    t2.join().unwrap();
    t3.join().unwrap();

    let done = *shared.done.lock().unwrap();
    println!("DONE done={}", done);
 cir_trace::finish();}
