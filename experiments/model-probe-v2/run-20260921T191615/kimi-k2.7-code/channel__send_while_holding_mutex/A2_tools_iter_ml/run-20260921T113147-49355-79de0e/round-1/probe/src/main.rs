use std::sync::{mpsc, Arc, Mutex};
use std::thread;

fn main() {
    let (tx, rx) = mpsc::sync_channel::<i32>(0);
    let shared = Arc::new(Mutex::new(0));
    let shared_s = Arc::clone(&shared);

    let s = thread::spawn(move || {
        {
            let mut guard = shared_s.lock().unwrap();
            *guard = 1;
        }
        tx.send(1).unwrap();
    });

    let r = thread::spawn(move || {
        let msg = rx.recv().unwrap();
        {
            let mut guard = shared.lock().unwrap();
            *guard = msg;
        }
    });

    s.join().unwrap();
    r.join().unwrap();

    let done = *shared.lock().unwrap();
    println!("DONE done={}", done);
}
