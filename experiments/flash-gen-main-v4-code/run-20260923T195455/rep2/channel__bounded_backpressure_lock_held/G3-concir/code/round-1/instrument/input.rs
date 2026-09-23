use std::sync::mpsc::{sync_channel, SyncSender, Receiver};
use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let m = Arc::new(Mutex::new(()));
    let (tx, rx): (SyncSender<i32>, Receiver<i32>) = sync_channel(1);

    let m_sender = Arc::clone(&m);
    let sender = thread::spawn(move || {
        {
            let _guard = m_sender.lock().unwrap();
        }
        tx.send(1).unwrap();
        {
            let _guard = m_sender.lock().unwrap();
        }
        tx.send(2).unwrap();
    });

    let m_receiver = Arc::clone(&m);
    let receiver = thread::spawn(move || {
        {
            let _guard = m_receiver.lock().unwrap();
        }
        let _x: i32 = rx.recv().unwrap();
        {
            let _guard = m_receiver.lock().unwrap();
        }
        let _y: i32 = rx.recv().unwrap();
    });

    sender.join().unwrap();
    receiver.join().unwrap();

    println!("DONE done=1");
}
