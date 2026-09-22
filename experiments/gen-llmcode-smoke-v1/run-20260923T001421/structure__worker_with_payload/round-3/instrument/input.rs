use std::sync::{Arc, Mutex};
use std::thread;

fn w2() {
    let local: i32 = 1;
    let _ = local;
}

fn w1(m: Arc<Mutex<i32>>) {
    {
        let mut acc = m.lock().unwrap();
        w2();
        let tmp = *acc;
        let tmp2 = tmp + 1;
        *acc = tmp2;
    }
}

fn main() {
    let m = Arc::new(Mutex::new(0));

    let m1 = Arc::clone(&m);
    let h1 = thread::spawn(move || {
        w1(m1);
    });

    let m2 = Arc::clone(&m);
    let h2 = thread::spawn(move || {
        w1(m2);
    });

    h1.join().unwrap();
    h2.join().unwrap();

    let done = *m.lock().unwrap();
    println!("DONE done={}", done);
}
