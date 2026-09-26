use std::sync::{mpsc, Arc, Mutex};
use std::thread;

struct Shared {
    done: i32,
}

fn sender(m: Arc<Mutex<Shared>>, tx: mpsc::SyncSender<i32>) {
    {
        let _guard = m.lock().unwrap();
    }
    tx.send(1).unwrap();
    {
        let _guard = m.lock().unwrap();
    }
    tx.send(2).unwrap();
}

fn receiver(m: Arc<Mutex<Shared>>, rx: mpsc::Receiver<i32>) {
    {
        let _guard = m.lock().unwrap();
    }
    let v1 = rx.recv().unwrap();
    {
        let _guard = m.lock().unwrap();
    }
    let v2 = rx.recv().unwrap();
    let _ = (v1, v2);
}

fn println() {
    println!("DONE done=1");
}

fn main() {
    let m = Arc::new(Mutex::new(Shared { done: 0 }));
    let ch = mpsc::sync_channel::<i32>(1);
    let (tx, rx) = ch;

    let m_sender = Arc::clone(&m);
    let m_receiver = Arc::clone(&m);

    let sender_thread = thread::spawn(move || {
        sender(m_sender, tx);
    });
    let receiver_thread = thread::spawn(move || {
        receiver(m_receiver, rx);
    });

    sender_thread.join().unwrap();
    receiver_thread.join().unwrap();

    {
        let mut guard = m.lock().unwrap();
        guard.done = 1;
    }

    println();
}
