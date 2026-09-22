use std::sync::{Arc, Mutex, mpsc};
use std::thread;

fn main() {
    let lock = Arc::new(Mutex::new(()));
    let (tx1, rx1) = mpsc::channel::<i32>();
    let (tx2, rx2) = mpsc::channel::<i32>();

    let lock_s = Arc::clone(&lock);
    let s = thread::spawn(move || {
        {
            let _g = lock_s.lock().unwrap();
        }
        tx1.send(1).unwrap();
        let v = rx2.recv().unwrap();
        {
            let _g = lock_s.lock().unwrap();
        }
        v
    });

    let lock_r = Arc::clone(&lock);
    let r = thread::spawn(move || {
        {
            let _g = lock_r.lock().unwrap();
        }
        let v = rx1.recv().unwrap();
        tx2.send(v).unwrap();
        {
            let _g = lock_r.lock().unwrap();
        }
        v
    });

    let sv = s.join().unwrap();
    let rv = r.join().unwrap();
    println!("DONE done={}", sv + rv - 1);
}
