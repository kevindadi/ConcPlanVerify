use std::sync::{mpsc, Arc, Mutex};
use std::thread;

fn sender(tx: mpsc::SyncSender<i32>, m: Arc<Mutex<()>>) {
    {
        let _guard = m.lock().unwrap();
    }
    tx.send(1).unwrap();
    {
        let _guard = m.lock().unwrap();
    }
    tx.send(2).unwrap();
}

fn receiver(rx: mpsc::Receiver<i32>, m: Arc<Mutex<()>>) {
    {
        let _guard = m.lock().unwrap();
    }
    let _ = rx.recv().unwrap();
    {
        let _guard = m.lock().unwrap();
    }
    let _ = rx.recv().unwrap();
}

fn main() {
    let m = Arc::new(Mutex::new(()));
    let (tx, rx) = mpsc::sync_channel::<i32>(1);

    let m_sender = Arc::clone(&m);
    let sender_handle = thread::spawn(move || sender(tx, m_sender));

    let m_receiver = Arc::clone(&m);
    let receiver_handle = thread::spawn(move || receiver(rx, m_receiver));

    sender_handle.join().unwrap();
    receiver_handle.join().unwrap();

    println!("DONE done=1");
}
