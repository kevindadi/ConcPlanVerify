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
        loop {
            let a = *announced;
            if a == 2 {
                break;
            }
            announced = shared.cv.wait(announced).unwrap();
        }
    }
    {
        let _m2_guard = shared.m2.lock().unwrap();
        shared.cv.notify_all();
    }
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

    let h1 = thread::spawn(move || waiter1(s1));
    let h2 = thread::spawn(move || waiter2(s2));
    let h3 = thread::spawn(move || notifier(s3));

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();

    let done = *shared.done.lock().unwrap();
    println!("DONE done={}", done);
}
