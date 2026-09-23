use concir_sync::Semaphore;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let m = Arc::new(Mutex::new(()));
    let (tx, rx): (SyncSender<i32>, Receiver<i32>) = sync_channel(1);

    let m_sender = Arc::clone(&m);
    let tx_sender = tx.clone();
    let sender = thread::spawn(move || {
        {
            let _guard = m_sender.lock().unwrap();
        }
        tx_sender.send(1).unwrap();
        {
            let _guard = m_sender.lock().unwrap();
        }
        tx_sender.send(2).unwrap();
    });

    let m_receiver = Arc::clone(&m);
    let receiver = thread::spawn(move || {
        {
            let _guard = m_receiver.lock().unwrap();
        }
        let _x = rx.recv().unwrap();
        {
            let _guard = m_receiver.lock().unwrap();
        }
        let _y = rx.recv().unwrap();
    });

    sender.join().unwrap();
    receiver.join().unwrap();

    println!("DONE done=1");
}
