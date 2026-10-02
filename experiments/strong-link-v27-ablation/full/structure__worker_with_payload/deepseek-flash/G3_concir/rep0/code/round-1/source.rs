use std::sync::{Arc, Mutex};
use std::thread;

fn compute() {
    let mut tmp = 0;
    tmp = 1;
    tmp = 2;
}

fn w1(m: &Mutex<i64>, acc: &Mutex<i64>) {
    {
        let mut acc_guard = acc.lock().unwrap();
        let _m_guard = m.lock().unwrap();
        compute();
        *acc_guard = 1;
    }
}

fn w2(m: &Mutex<i64>, acc: &Mutex<i64>) {
    {
        let mut acc_guard = acc.lock().unwrap();
        let _m_guard = m.lock().unwrap();
        compute();
        *acc_guard = 1;
    }
}

fn main() {
    let m = Arc::new(Mutex::new(()));
    let acc = Arc::new(Mutex::new(0i64));

    let m1 = Arc::clone(&m);
    let a1 = Arc::clone(&acc);
    let t1 = thread::spawn(move || {
        w1(&m1, &a1);
    });

    let m2 = Arc::clone(&m);
    let a2 = Arc::clone(&acc);
    let t2 = thread::spawn(move || {
        w2(&m2, &a2);
    });

    t1.join().unwrap();
    t2.join().unwrap();

    let value = *acc.lock().unwrap();
    println!("DONE done={}", value);
}
