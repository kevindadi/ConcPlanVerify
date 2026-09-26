use concir_sync::Semaphore;
use std::sync::mpsc::sync_channel;
use std::sync::Arc;
use std::thread;

fn main() {
    let ch1 = sync_channel::<i32>(0);
    let ch2 = sync_channel::<i32>(0);
    let lock = Semaphore::new(1);

    let (ch1_tx, ch1_rx) = ch1;
    let (ch2_tx, ch2_rx) = ch2;
    let lock_s = Arc::clone(&lock);
    let lock_r = Arc::clone(&lock);

    let s = thread::spawn(move || {
        let permit = lock_s.acquire();
        permit.release();
        ch1_tx.send(1).unwrap();
        ch2_rx.recv().unwrap();
    });

    let r = thread::spawn(move || {
        let permit = lock_r.acquire();
        permit.release();
        let done = ch1_rx.recv().unwrap();
        ch2_tx.send(done).unwrap();
        done
    });

    s.join().unwrap();
    let done = r.join().unwrap();
    println!("DONE done={done}");
}
