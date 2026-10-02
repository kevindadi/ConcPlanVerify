use std::sync::{Arc, Mutex};
use std::sync::mpsc::{sync_channel, SyncSender, Receiver};
use std::thread;

fn sender(ch: SyncSender<i32>, m: Arc<Mutex<()>>) {
    {
        let _guard = m.lock().unwrap();
    }
    ch.send(1).unwrap();
    {
        let _guard = m.lock().unwrap();
    }
    ch.send(2).unwrap();
}

fn receiver(ch: Receiver<i32>, m: Arc<Mutex<()>>) {
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
        sender(tx, m_sender);
    });

    let m_receiver = Arc::clone(&m);
    let receiver_handle = thread::spawn(move || {
        receiver(rx, m_receiver);
    });

    sender_handle.join().unwrap();
    receiver_handle.join().unwrap();

    println!("DONE done=1");
}
