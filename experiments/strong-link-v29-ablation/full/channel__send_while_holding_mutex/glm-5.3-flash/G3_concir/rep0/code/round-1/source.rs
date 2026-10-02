use std::sync::{Arc, Mutex};
use std::sync::mpsc::sync_channel;
use std::thread;

fn s(lock: &Arc<Mutex<()>>, ch1_tx: std::sync::mpsc::SyncSender<i32>, ch2_rx: std::sync::mpsc::Receiver<i32>) {
    let _g = lock.lock().unwrap();
    drop(_g);
    ch1_tx.send(1).unwrap();
    let _ = ch2_rx.recv().unwrap();
    let _g = lock.lock().unwrap();
    drop(_g);
}

fn r(lock: &Arc<Mutex<()>>, ch1_rx: std::sync::mpsc::Receiver<i32>, ch2_tx: std::sync::mpsc::SyncSender<i32>) {
    let _g = lock.lock().unwrap();
    drop(_g);
    let _ = ch1_rx.recv().unwrap();
    ch2_tx.send(1).unwrap();
    let _g = lock.lock().unwrap();
    drop(_g);
}

fn main() {
    let lock = Arc::new(Mutex::new(()));
    let (ch1_tx, ch1_rx) = sync_channel::<i32>(0);
    let (ch2_tx, ch2_rx) = sync_channel::<i32>(0);

    let lock_s = Arc::clone(&lock);
    let hs = thread::spawn(move || {
        s(&lock_s, ch1_tx, ch2_rx);
    });

    let lock_r = Arc::clone(&lock);
    let hr = thread::spawn(move || {
        r(&lock_r, ch1_rx, ch2_tx);
    });

    hs.join().unwrap();
    hr.join().unwrap();

    println!("DONE done=1");
}
