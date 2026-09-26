use concir_sync::Semaphore;
use std::sync::{mpsc::sync_channel, Arc};
use std::thread;

fn s(
    ch1: std::sync::mpsc::SyncSender<i32>,
    ch2: std::sync::mpsc::Receiver<i32>,
    lock: Arc<Semaphore>,
) {
    {
        let _permit = lock.acquire();
    }
    ch1.send(1).unwrap();
    let _ = ch2.recv().unwrap();
}

fn r(
    ch1: std::sync::mpsc::Receiver<i32>,
    ch2: std::sync::mpsc::SyncSender<i32>,
    lock: Arc<Semaphore>,
) {
    let _ = ch1.recv().unwrap();
    {
        let _permit = lock.acquire();
    }
    ch2.send(0).unwrap();
}

fn main() {
    let lock = Semaphore::new(1);
    let ch1 = sync_channel::<i32>(0);
    let ch2 = sync_channel::<i32>(0);

    let lock_s = Arc::clone(&lock);
    let lock_r = Arc::clone(&lock);

    let ch1_tx = ch1.0;
    let ch1_rx = ch1.1;
    let ch2_tx = ch2.0;
    let ch2_rx = ch2.1;

    let ts = thread::spawn(move || s(ch1_tx, ch2_rx, lock_s));
    let tr = thread::spawn(move || r(ch1_rx, ch2_tx, lock_r));

    ts.join().unwrap();
    tr.join().unwrap();
    println!("DONE done=1");
}
