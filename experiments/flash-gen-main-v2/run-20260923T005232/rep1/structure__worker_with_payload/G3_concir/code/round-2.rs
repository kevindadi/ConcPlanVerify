use std::sync::{Arc, Mutex};
use std::thread;

fn compute() {
    let x = 1;
    let _y = x + 1;
}

fn w1(m: Arc<Mutex<i32>>, acc: Arc<Mutex<i32>>) {
    {
        let mut guard = m.lock().unwrap();
        compute();
        let mut tmp = acc.lock().unwrap();
        *tmp = *tmp + 1;
        let _ = &mut guard;
    }
}

fn w2(m: Arc<Mutex<i32>>, acc: Arc<Mutex<i32>>) {
    {
        let mut guard = m.lock().unwrap();
        compute();
        let mut tmp = acc.lock().unwrap();
        *tmp = *tmp + 1;
        let _ = &mut guard;
    }
}

fn main() {
    let m = Arc::new(Mutex::new(0));
    let acc = Arc::new(Mutex::new(0));

    let m1 = Arc::clone(&m);
    let acc1 = Arc::clone(&acc);
    let h1 = thread::spawn(move || w1(m1, acc1));

    let m2 = Arc::clone(&m);
    let acc2 = Arc::clone(&acc);
    let h2 = thread::spawn(move || w2(m2, acc2));

    h1.join().unwrap();
    h2.join().unwrap();

    let done = *acc.lock().unwrap();
    println!("DONE done={}", done);
}
