use std::sync::{mpsc, Arc, Mutex};
use std::thread;

fn main() {
    let (tx, rx) = mpsc::sync_channel::<i32>(0);
    let shared = Arc::new(Mutex::new(0));

    let shared_s = Arc::clone(&shared);
    let s = thread::spawn(move || {
        {
            let mut data = shared_s.lock().unwrap();
            *data += 1;
        }
        tx.send(1).unwrap();
    });

    let shared_r = Arc::clone(&shared);
    let r = thread::spawn(move || {
        let _ = rx.recv().unwrap();
        {
            let mut data = shared_r.lock().unwrap();
            *data += 1;
        }
    });

    s.join().unwrap();
    r.join().unwrap();

    assert_eq!(*shared.lock().unwrap(), 2);
    println!("DONE done=1");
}
