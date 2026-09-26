use std::sync::{Arc, Mutex};
use std::thread;

use concir_sync::Semaphore;

fn w1(s: Arc<Semaphore>, done: Arc<Mutex<i32>>) {
    let permit = s.acquire();
    {
        let mut slot = done.lock().unwrap();
        *slot = 1;
    }
    permit.release();

    let permit = s.acquire();
    {
        let mut slot = done.lock().unwrap();
        *slot = 1;
    }
    permit.release();
}

fn w2(s: Arc<Semaphore>, done: Arc<Mutex<i32>>) {
    let permit = s.acquire();
    {
        let mut slot = done.lock().unwrap();
        *slot = 1;
    }
    permit.release();

    let permit = s.acquire();
    {
        let mut slot = done.lock().unwrap();
        *slot = 1;
    }
    permit.release();
}

fn main() {
    let s = Semaphore::new(1);
    let done = Arc::new(Mutex::new(0));

    let s1 = Arc::clone(&s);
    let done1 = Arc::clone(&done);
    let t1 = thread::spawn(move || w1(s1, done1));

    let s2 = Arc::clone(&s);
    let done2 = Arc::clone(&done);
    let t2 = thread::spawn(move || w2(s2, done2));

    t1.join().unwrap();
    t2.join().unwrap();

    let done_value = *done.lock().unwrap();
    println!("DONE done={}", done_value);
}
