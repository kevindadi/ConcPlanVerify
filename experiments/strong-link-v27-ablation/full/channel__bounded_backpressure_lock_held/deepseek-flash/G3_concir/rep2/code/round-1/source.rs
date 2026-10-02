use std::sync::{Arc, Mutex};
use std::sync::mpsc;
use std::thread;

fn sender(m: Arc<Mutex<()>>, ch: mpsc::SyncSender<i32>) {
    let guard = m.lock().unwrap();
    drop(guard);

    ch.send(1).unwrap();
    ch.send(2).unwrap();
}

fn receiver(m: Arc<Mutex<()>>, ch: mpsc::Receiver<i32>) {
    let guard = m.lock().unwrap();
    drop(guard);

    let mut v1 = 0;
    let mut v2 = 0;
    v1 = ch.recv().unwrap();
    v2 = ch.recv().unwrap();
    let _ = (v1, v2);
}

fn main() {
    let m = Arc::new(Mutex::new(()));
    let (ch_tx, ch_rx) = mpsc::sync_channel::<i32>(1);

    let m_sender = Arc::clone(&m);
    let sender_handle = thread::spawn(move || sender(m_sender, ch_tx));

    let m_receiver = Arc::clone(&m);
    let receiver_handle = thread::spawn(move || receiver(m_receiver, ch_rx));

    sender_handle.join().unwrap();
    receiver_handle.join().unwrap();

    println!("DONE done=1");
}
