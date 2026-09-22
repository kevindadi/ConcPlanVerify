use std::sync::{Arc, Mutex};
use std::thread;

fn compute() {
    let x: i32 = 1;
    let _y: i32 = x + 1;
}

fn w1(m: Arc<Mutex<()>>, acc: Arc<Mutex<i32>>) {
    {
        let _guard = m.lock().unwrap();
        compute();
        let tmp = *acc.lock().unwrap();
        let tmp2 = tmp + 1;
        *acc.lock().unwrap() = tmp2;
    }
}

fn w2(m: Arc<Mutex<()>>, acc: Arc<Mutex<i32>>) {
    {
        let _guard = m.lock().unwrap();
        compute();
        let tmp = *acc.lock().unwrap();
        let tmp2 = tmp + 1;
        *acc.lock().unwrap() = tmp2;
    }
}

fn main() {
    let m = Arc::new(Mutex::new(()));
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
