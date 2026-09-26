use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Arc, Mutex};
use std::thread;

fn s(m: Arc<Mutex<i32>>, ch1_tx: SyncSender<i32>, ch2_rx: Receiver<i32>) {
    {
        let mut n = m.lock().unwrap();
        *n = 1;
    }
    ch1_tx.send(1).unwrap();
    let ack = ch2_rx.recv().unwrap();
    let _ = ack;
}

fn r(m: Arc<Mutex<i32>>, ch1_rx: Receiver<i32>, ch2_tx: SyncSender<i32>) {
    {
        let mut n = m.lock().unwrap();
        *n = 2;
    }
    let v = ch1_rx.recv().unwrap();
    let _ = v;
    ch2_tx.send(1).unwrap();
}

fn main() {
    let m = Arc::new(Mutex::new(0i32));
    let (ch1_tx, ch1_rx) = sync_channel::<i32>(0);
    let (ch2_tx, ch2_rx) = sync_channel::<i32>(0);

    let m_s = Arc::clone(&m);
    let m_r = Arc::clone(&m);

    let handle_s = thread::spawn(move || s(m_s, ch1_tx, ch2_rx));
    let handle_r = thread::spawn(move || r(m_r, ch1_rx, ch2_tx));

    handle_s.join().unwrap();
    handle_r.join().unwrap();

    println!("DONE done=1");
}
