use std::sync::{mpsc, Arc, Mutex};
use std::thread;

fn main() {
    // ch: a channel that holds at most one value (capacity 1).
    let (tx, rx) = mpsc::sync_channel::<i32>(1);
    // m: shared lock both roles occasionally need.
    let m: Arc<Mutex<usize>> = Arc::new(Mutex::new(0));

    let sender = {
        let m = Arc::clone(&m);
        thread::spawn(move || {
            for v in 1..=2 {
                // R4/R5: block on the full channel WITHOUT holding m.
                tx.send(v).expect("receiver alive");
                let mut cnt = m.lock().unwrap();
                *cnt += 1;
                drop(cnt);
            }
        })
    };

    let receiver = {
        let m = Arc::clone(&m);
        thread::spawn(move || {
            loop {
                // R4/R5: block on the empty channel WITHOUT holding m.
                match rx.recv() {
                    Ok(_v) => {
                        let mut cnt = m.lock().unwrap();
                        *cnt += 1;
                        drop(cnt);
                    }
                    Err(_) => break,
                }
            }
        })
    };

    sender.join().expect("sender thread");
    receiver.join().expect("receiver thread");

    let cnt = m.lock().unwrap();
    let _ = *cnt; // both roles finished after two values passed (R3)
    drop(cnt);
    println!("DONE done=1");
}
