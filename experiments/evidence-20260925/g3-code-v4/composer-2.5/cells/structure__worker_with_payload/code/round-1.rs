use std::sync::{Arc, Mutex};
use std::thread;

fn compute() -> i32 {
    let payload = 1;
    let result = payload;
    result
}

fn w1(m: Arc<Mutex<i32>>, acc: Arc<Mutex<i32>>) {
    let _guard = m.lock().unwrap();
    let _ = compute();
    {
        let mut a = acc.lock().unwrap();
        *a = *a + 1;
    }
}

fn w2(m: Arc<Mutex<i32>>, acc: Arc<Mutex<i32>>) {
    let _guard = m.lock().unwrap();
    let _ = compute();
    {
        let mut a = acc.lock().unwrap();
        *a = *a + 1;
    }
}

fn main() {
    let m = Arc::new(Mutex::new(()));
    let acc = Arc::new(Mutex::new(0i32));

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
