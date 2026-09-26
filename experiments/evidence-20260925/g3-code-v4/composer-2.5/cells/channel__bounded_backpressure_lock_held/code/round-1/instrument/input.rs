use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;

fn sender(ch: mpsc::SyncSender<i32>, m: Arc<Mutex<()>>) {
    ch.send(1).unwrap();
    let _guard = m.lock().unwrap();
    drop(_guard);
    ch.send(2).unwrap();
}

fn receiver(ch: mpsc::Receiver<i32>, m: Arc<Mutex<()>>) {
    let v1 = ch.recv().unwrap();
    let _ = v1;
    let _guard = m.lock().unwrap();
    drop(_guard);
    let v2 = ch.recv().unwrap();
    let _ = v2;
}

fn main() {
    let m = Arc::new(Mutex::new(()));
    let (ch_tx, ch_rx) = mpsc::sync_channel::<i32>(1);

    let m_sender = Arc::clone(&m);
    let ch_sender = ch_tx;
    let sender_handle = thread::spawn(move || sender(ch_sender, m_sender));

    let m_receiver = Arc::clone(&m);
    let ch_receiver = ch_rx;
    let receiver_handle = thread::spawn(move || receiver(ch_receiver, m_receiver));

    sender_handle.join().unwrap();
    receiver_handle.join().unwrap();

    println!("DONE done=1");
}
