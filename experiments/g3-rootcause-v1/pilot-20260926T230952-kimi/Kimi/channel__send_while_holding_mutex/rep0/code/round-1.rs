use std::sync::{Arc, Mutex};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::thread;

fn s(m: Arc<Mutex<i32>>, ch1_tx: SyncSender<i32>, ch2_rx: Receiver<i32>) {
    {
        let mut done = m.lock().unwrap();
        *done = 1;
    }
    ch1_tx.send(1).unwrap();
    let _ = ch2_rx.recv().unwrap();
}

fn r(m: Arc<Mutex<i32>>, ch1_rx: Receiver<i32>, ch2_tx: SyncSender<i32>) {
    {
        let done = m.lock().unwrap();
        let _ = *done;
    }
    let v = ch1_rx.recv().unwrap();
    ch2_tx.send(v).unwrap();
}

fn main() {
    let m = Arc::new(Mutex::new(0i32));
    let (ch1_tx, ch1_rx) = sync_channel::<i32>(0);
    let (ch2_tx, ch2_rx) = sync_channel::<i32>(0);

    let m_s = Arc::clone(&m);
    let m_r = Arc::clone(&m);

    let hs = thread::spawn(move || s(m_s, ch1_tx, ch2_rx));
    let hr = thread::spawn(move || r(m_r, ch1_rx, ch2_tx));

    hs.join().unwrap();
    hr.join().unwrap();

    let done = m.lock().unwrap();
    println!("DONE done={}", *done);
}
