use std::sync::{Arc, Mutex};
use std::thread;

fn compute() {
    let x = 1;
    let _y = x + 1;
}

fn w1(m: Arc<Mutex<i32>>) {
    {
        let mut acc_kept = m.lock().unwrap();
        compute();
        let tmp = *acc_kept;
        let tmp = tmp + 1;
        *acc_kept = tmp;
    }
}

fn w2(m: Arc<Mutex<i32>>) {
    {
        let mut acc_kept = m.lock().unwrap();
        compute();
        let tmp = *acc_kept;
        let tmp = tmp + 1;
        *acc_kept = tmp;
    }
}

fn main() {
    let m = Arc::new(Mutex::new(0));

    let m1 = Arc::clone(&m);
    let h1 = thread::spawn(move || w1(m1));

    let m2 = Arc::clone(&m);
    let h2 = thread::spawn(move || w2(m2));

    h1.join().unwrap();
    h2.join().unwrap();

    let done = *m.lock().unwrap();
    println!("DONE done={}", done);
}
