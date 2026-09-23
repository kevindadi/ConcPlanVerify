use std::sync::{Arc, Condvar, Mutex};
use std::sync::mpsc::{sync_channel, SyncSender, Receiver};
use std::thread;

struct Channel {
    tx: SyncSender<i32>,
    rx: Mutex<Option<Receiver<i32>>>,
}

fn main() {
    let m = Arc::new(Mutex::new(()));
    let (tx, rx) = sync_channel::<i32>(1);
    let ch = Arc::new(Channel {
        tx,
        rx: Mutex::new(Some(rx)),
    });

    let m_sender = Arc::clone(&m);
    let ch_sender = Arc::clone(&ch);
    let sender = thread::spawn(move || {
        {
            let _g = m_sender.lock().unwrap();
        }
        ch_sender.tx.send(1).unwrap();
        {
            let _g = m_sender.lock().unwrap();
        }
        ch_sender.tx.send(2).unwrap();
    });

    let m_receiver = Arc::clone(&m);
    let ch_receiver = Arc::clone(&ch);
    let receiver = thread::spawn(move || {
        let rx = ch_receiver.rx.lock().unwrap().take().unwrap();
        {
            let _g = m_receiver.lock().unwrap();
        }
        let _x: i32 = rx.recv().unwrap();
        {
            let _g = m_receiver.lock().unwrap();
        }
        let _y: i32 = rx.recv().unwrap();
    });

    sender.join().unwrap();
    receiver.join().unwrap();

    println!("DONE done=1");
}
