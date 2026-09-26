use concir_sync::Semaphore;
use std::sync::{Arc, mpsc::sync_channel};
use std::thread;

fn main() {
    let slots = Semaphore::new(2);
    let slots_s1 = Arc::clone(&slots);
    let slots_r = slots;

    let ch = sync_channel::<i32>(0);
    let (tx, rx) = ch;

    let s1 = thread::spawn(move || {
        let _permit = slots_s1.acquire();
        tx.send(1).unwrap();
    });

    let r = thread::spawn(move || {
        let _permit = slots_r.acquire();
        rx.recv().unwrap()
    });

    s1.join().unwrap();
    let done = r.join().unwrap();
    println!("DONE done={}", done);
}
