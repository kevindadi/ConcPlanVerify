use std::sync::{Arc, Mutex};
use std::sync::mpsc::sync_channel;
use std::thread;

fn s(m: &Arc<Mutex<()>>, ch1_tx: &std::sync::mpsc::SyncSender<i32>, ch2_rx: &std::sync::mpsc::Receiver<i32>) {
    let _guard = m.lock().unwrap();
    drop(_guard);
    ch1_tx.send(1).unwrap();
    let _ = ch2_rx.recv().unwrap();
}

fn r(m: &Arc<Mutex<()>>, ch1_rx: &std::sync::mpsc::Receiver<i32>, ch2_tx: &std::sync::mpsc::SyncSender<i32>) {
    let _guard = m.lock().unwrap();
    drop(_guard);
    let _ = ch1_rx.recv().unwrap();
    ch2_tx.send(1).unwrap();
}

fn main() {
    let m = Arc::new(Mutex::new(()));
    let (ch1_tx, ch1_rx) = sync_channel::<i32>(0);
    let (ch2_tx, ch2_rx) = sync_channel::<i32>(0);

    let m_s = Arc::clone(&m);
    let ch1_tx_s = ch1_tx.clone();
    let ch2_rx_s = ch2_rx;
    let handle_s = thread::spawn(move || {
        s(&m_s, &ch1_tx_s, &ch2_rx_s);
    });

    let m_r = Arc::clone(&m);
    let ch1_rx_r = ch1_rx;
    let ch2_tx_r = ch2_tx;
    let handle_r = thread::spawn(move || {
        r(&m_r, &ch1_rx_r, &ch2_tx_r);
    });

    handle_s.join().unwrap();
    handle_r.join().unwrap();

    println!("DONE done=1");
}
