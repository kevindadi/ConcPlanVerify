use std::sync::{Arc, Mutex};
use std::thread;

static mut ACC: i32 = 0;

fn w2() -> i32 {
    let local = 1;
    local
}

fn w1(m: &Mutex<i32>) {
    {
        let mut acc = m.lock().unwrap();
        let _ = w2();
        let tmp = *acc;
        let tmp2 = tmp + 1;
        *acc = tmp2;
    }
}

fn main() {
    let m = Arc::new(Mutex::new(0i32));

    let m1 = Arc::clone(&m);
    let h1 = thread::spawn(move || {
        w1(&m1);
    });

    let m2 = Arc::clone(&m);
    let h2 = thread::spawn(move || {
        w1(&m2);
    });

    h1.join().unwrap();
    h2.join().unwrap();

    let done = *m.lock().unwrap();
    println!("DONE done={}", done);
}
