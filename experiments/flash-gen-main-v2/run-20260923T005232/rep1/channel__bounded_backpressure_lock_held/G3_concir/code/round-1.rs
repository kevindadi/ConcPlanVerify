use std::sync::{Arc, Mutex, Condvar};
use std::sync::mpsc::{sync_channel, SyncSender, Receiver};
use std::thread;

struct Channel {
    sender: SyncSender<i32>,
    receiver: Receiver<i32>,
}

fn main() {
    let m = Arc::new(Mutex::new(()));
    let (tx, rx) = sync_channel::<i32>(1);
    let ch = Arc::new(Channel { sender: tx, receiver: rx });

    let m_sender = Arc::clone(&m);
    let ch_sender = Arc::clone(&ch);
    let sender = thread::spawn(move || {
        {
            let _guard = m_sender.lock().unwrap();
        }
        ch_sender.sender.send(1).unwrap();
        {
            let _guard = m_sender.lock().unwrap();
        }
        ch_sender.sender.send(2).unwrap();
    });

    let m_receiver = Arc::clone(&m);
    let ch_receiver = Arc::clone(&ch);
    let receiver = thread::spawn(move || {
        {
            let _guard = m_receiver.lock().unwrap();
        }
        let x = ch_receiver.receiver.recv().unwrap();
        {
            let _guard = m_receiver.lock().unwrap();
        }
        let y = ch_receiver.receiver.recv().unwrap();
        let _ = (x, y);
    });

    sender.join().unwrap();
    receiver.join().unwrap();

    println!("DONE done=1");
}
