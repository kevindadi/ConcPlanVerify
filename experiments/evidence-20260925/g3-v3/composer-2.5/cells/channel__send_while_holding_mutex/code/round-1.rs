use std::sync::mpsc::sync_channel;
use std::sync::{Arc, Mutex};
use std::thread;

fn s(lk: Arc<Mutex<()>>, ch1_tx: std::sync::mpsc::SyncSender<i32>, ch2_rx: std::sync::mpsc::Receiver<i32>) {
    {
        let _guard = lk.lock().unwrap();
    }
    ch1_tx.send(1).unwrap();
    let _v = ch2_rx.recv().unwrap();
    {
        let _guard = lk.lock().unwrap();
    }
}

fn r(lk: Arc<Mutex<()>>, ch1_rx: std::sync::mpsc::Receiver<i32>, ch2_tx: std::sync::mpsc::SyncSender<i32>) {
    {
        let _guard = lk.lock().unwrap();
    }
    let _v = ch1_rx.recv().unwrap();
    ch2_tx.send(1).unwrap();
    {
        let _guard = lk.lock().unwrap();
    }
}

fn main() {
    let lk = Arc::new(Mutex::new(()));
    let (ch1_tx, ch1_rx) = sync_channel::<i32>(0);
    let (ch2_tx, ch2_rx) = sync_channel::<i32>(0);

    let lk_s = Arc::clone(&lk);
    let s_handle = thread::spawn(move || s(lk_s, ch1_tx, ch2_rx));

    let lk_r = Arc::clone(&lk);
    let r_handle = thread::spawn(move || r(lk_r, ch1_rx, ch2_tx));

    s_handle.join().unwrap();
    r_handle.join().unwrap();

    let done = 1;
    println!("DONE done={}", done);
}
