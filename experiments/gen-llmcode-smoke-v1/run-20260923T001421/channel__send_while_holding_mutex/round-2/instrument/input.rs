use std::sync::{Arc, Mutex};
use std::sync::mpsc::{sync_channel, SyncSender, Receiver};
use std::thread;

fn main() {
    let lock = Arc::new(Mutex::new(()));
    let (tx, rx): (SyncSender<i32>, Receiver<i32>) = sync_channel(0);
    let done = Arc::new(Mutex::new(0i32));

    let lock_s = Arc::clone(&lock);
    let tx_s = tx.clone();
    let s = thread::spawn(move || {
        {
            let _g = lock_s.lock().unwrap();
        }
        tx_s.send(1).unwrap();
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
