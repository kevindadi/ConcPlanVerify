use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Arc, Mutex};
use std::thread;

fn sender(m: &Mutex<()>, ch: &SyncSender<i32>) {
    {
        let _guard = m.lock().unwrap();
    }
    ch.send(1).unwrap();
    {
        let _guard = m.lock().unwrap();
    }
    ch.send(2).unwrap();
}

fn receiver(m: &Mutex<()>, ch: &Receiver<i32>) {
    {
        let _guard = m.lock().unwrap();
    }
    let x = ch.recv().unwrap();
    {
        let _guard = m.lock().unwrap();
    }
    let y = ch.recv().unwrap();
    let _ = (x, y);
}

fn main() {
    let m = Arc::new(Mutex::new(()));
    let (tx, rx) = sync_channel::<i32>(1);

    let m_sender = Arc::clone(&m);
    let sender_handle = thread::spawn(move || {
        sender(&m_sender, &tx);
    });

    let m_receiver = Arc::clone(&m);
    let receiver_handle = thread::spawn(move || {
        receiver(&m_receiver, &rx);
    });

    sender_handle.join().unwrap();
    receiver_handle.join().unwrap();

    println!("DONE done=1");
}
