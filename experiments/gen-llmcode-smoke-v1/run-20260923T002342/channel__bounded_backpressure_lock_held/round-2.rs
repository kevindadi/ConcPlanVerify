use std::sync::{mpsc, Arc, Mutex};
use std::thread;

fn main() {
    let m = Arc::new(Mutex::new(()));
    let (tx, rx) = mpsc::sync_channel::<i32>(1);

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
        let _v1 = rx.recv().unwrap();
        {
            let _guard = m_receiver.lock().unwrap();
        }
        let _v2 = rx.recv().unwrap();
    });

    sender.join().unwrap();
    receiver.join().unwrap();

    println!("DONE done=1");
}
