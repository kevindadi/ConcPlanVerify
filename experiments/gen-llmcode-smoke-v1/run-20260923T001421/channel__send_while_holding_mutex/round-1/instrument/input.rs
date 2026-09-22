use std::sync::{mpsc, Arc, Mutex};
use std::thread;

fn main() {
    let lock = Arc::new(Mutex::new(()));
    let done = Arc::new(Mutex::new(0i32));
    let (tx, rx) = mpsc::sync_channel::<i32>(0);

    let lock_s = Arc::clone(&lock);
    let s = thread::spawn(move || {
        {
            let _g = lock_s.lock().unwrap();
        }
        tx.send(1).unwrap();
        {
            let _g = lock_s.lock().unwrap();
        }
    });

    let lock_r = Arc::clone(&lock);
    let done_r = Arc::clone(&done);
    let r = thread::spawn(move || {
        {
            let _g = lock_r.lock().unwrap();
        }
        let _v: i32 = rx.recv().unwrap();
        {
            let _g = lock_r.lock().unwrap();
        }
        {
            let mut d = done_r.lock().unwrap();
            *d = 1;
        }
    });

    s.join().unwrap();
    r.join().unwrap();

    let d = *done.lock().unwrap();
    println!("DONE done={}", d);
}
