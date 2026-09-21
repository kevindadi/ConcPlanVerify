use std::sync::{mpsc, Arc, Mutex};
use std::thread;

fn main() {
    let (tx, rx) = mpsc::sync_channel::<i32>(1);
    let m = Arc::new(Mutex::new(()));
    let m1 = Arc::clone(&m);
    let s = thread::spawn(move || {
        {
            let _g = m1.lock().unwrap();
            // lock is only held for local work, never across channel ops
        }
        tx.send(1).unwrap();
        tx.send(2).unwrap();
    });
    let r = thread::spawn(move || {
        {
            let _g = m.lock().unwrap();
            // lock is only held for local work, never across channel ops
        }
        rx.recv().unwrap();
        rx.recv().unwrap();
    });
    s.join().unwrap();
    r.join().unwrap();
    println!("DONE done=1");
}
