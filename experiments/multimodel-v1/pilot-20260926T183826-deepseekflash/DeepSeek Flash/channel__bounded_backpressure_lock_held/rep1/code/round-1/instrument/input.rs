use std::sync::{mpsc, Arc, Mutex};
use std::thread;

fn main() {
    let m = Arc::new(Mutex::new(()));
    let (tx, rx) = mpsc::sync_channel::<i32>(1);

    let m_sender = Arc::clone(&m);
    let sender = thread::spawn(move || {
        sender(m_sender, tx);
    });

    let m_receiver = Arc::clone(&m);
    let receiver = thread::spawn(move || {
        receiver(m_receiver, rx);
    });

    sender.join().unwrap();
    receiver.join().unwrap();

    println!("DONE done=1");
}

fn sender(m: Arc<Mutex<()>>, tx: mpsc::SyncSender<i32>) {
    {
        let _guard = m.lock().unwrap();
    }
    send(&tx, 1);
    {
        let _guard = m.lock().unwrap();
    }
    send(&tx, 2);
}

fn receiver(m: Arc<Mutex<()>>, rx: mpsc::Receiver<i32>) {
    {
        let _guard = m.lock().unwrap();
    }
    recv(&rx);
    {
        let _guard = m.lock().unwrap();
    }
    recv(&rx);
}

fn send(tx: &mpsc::SyncSender<i32>, v: i32) {
    tx.send(v).unwrap();
}

fn recv(rx: &mpsc::Receiver<i32>) {
    let _v = rx.recv().unwrap();
}
