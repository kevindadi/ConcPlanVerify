use std::sync::{mpsc, Arc, Mutex};
use std::thread;

fn main() {
    let ch = mpsc::sync_channel::<i32>(1);
    let m = Arc::new(Mutex::new(()));

    let ch_sender = ch.0;
    let m_sender = Arc::clone(&m);
    let sender = thread::spawn(move || {
        {
            let _guard = m_sender.lock().unwrap();
        }
        ch_sender.send(1).unwrap();
        {
            let _guard = m_sender.lock().unwrap();
        }
        ch_sender.send(2).unwrap();
    });

    let ch_receiver = ch.1;
    let m_receiver = Arc::clone(&m);
    let receiver = thread::spawn(move || {
        {
            let _guard = m_receiver.lock().unwrap();
        }
        let _v1 = ch_receiver.recv().unwrap();
        {
            let _guard = m_receiver.lock().unwrap();
        }
        let _v2 = ch_receiver.recv().unwrap();
    });

    sender.join().unwrap();
    receiver.join().unwrap();

    println!("DONE done=1");
}
