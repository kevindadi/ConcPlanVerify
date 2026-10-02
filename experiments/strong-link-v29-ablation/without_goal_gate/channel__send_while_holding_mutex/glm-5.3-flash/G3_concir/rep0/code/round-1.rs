use std::sync::{Arc, Mutex};
use std::sync::mpsc::sync_channel;
use std::thread;

fn s(
    ch1_tx: std::sync::mpsc::SyncSender<i32>,
    ch2_rx: std::sync::mpsc::Receiver<i32>,
    m: &Arc<Mutex<i32>>,
) {
    {
        let _guard = m.lock().unwrap();
    }
    let _ = ch1_tx.send(1);
    let got = ch2_rx.recv().unwrap();
    let _ = got;
}

fn r(
    ch1_rx: std::sync::mpsc::Receiver<i32>,
    ch2_tx: std::sync::mpsc::SyncSender<i32>,
    m: &Arc<Mutex<i32>>,
) {
    {
        let _guard = m.lock().unwrap();
    }
    let got = ch1_rx.recv().unwrap();
    let _ = ch2_tx.send(1);
    let _ = got;
}

fn main() {
    let m = Arc::new(Mutex::new(0));

    let (ch1_tx, ch1_rx) = sync_channel::<i32>(0);
    let (ch2_tx, ch2_rx) = sync_channel::<i32>(0);

    let m_s = Arc::clone(&m);
    let h_s = thread::spawn(move || s(ch1_tx, ch2_rx, &m_s));

    let m_r = Arc::clone(&m);
    let h_r = thread::spawn(move || r(ch1_rx, ch2_tx, &m_r));

    h_s.join().unwrap();
    h_r.join().unwrap();

    let done = *m.lock().unwrap();
    let _ = done;
    println!("DONE done=1");
}
