use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Arc, Mutex};
use std::thread;

fn s(m: Arc<Mutex<()>>, ch1_tx: SyncSender<i32>, ch2_rx: Receiver<i32>) {
    let guard = m.lock().unwrap();
    drop(guard);

    ch1_tx.send(1).unwrap();
    let reply = ch2_rx.recv().unwrap();
    let _ = reply;
}

fn r(m: Arc<Mutex<()>>, ch1_rx: Receiver<i32>, ch2_tx: SyncSender<i32>) {
    let guard = m.lock().unwrap();
    drop(guard);

    let message = ch1_rx.recv().unwrap();
    ch2_tx.send(message).unwrap();
}

fn main() {
    let m = Arc::new(Mutex::new(()));

    let (ch1_tx, ch1_rx) = sync_channel(0);
    let (ch2_tx, ch2_rx) = sync_channel(0);

    let s_m = Arc::clone(&m);
    let s_thread = thread::spawn(move || s(s_m, ch1_tx, ch2_rx));

    let r_m = Arc::clone(&m);
    let r_thread = thread::spawn(move || r(r_m, ch1_rx, ch2_tx));

    s_thread.join().unwrap();
    r_thread.join().unwrap();

    println!("DONE done=1");
}
