use concir_sync::Semaphore;
use std::sync::mpsc::sync_channel;
use std::sync::Arc;
use std::thread;

fn sender(ch: std::sync::mpsc::SyncSender<i32>, m: Arc<Semaphore>) {
    {
        let _permit = m.acquire();
        let _ = _permit;
    }
    ch.send(1).unwrap();
    {
        let _permit = m.acquire();
        let _ = _permit;
    }
    ch.send(2).unwrap();
}

fn receiver(ch: std::sync::mpsc::Receiver<i32>, m: Arc<Semaphore>) {
    let v1 = ch.recv().unwrap();
    {
        let _permit = m.acquire();
        let _ = v1;
    }
    let v2 = ch.recv().unwrap();
    {
        let _permit = m.acquire();
        let _ = v2;
    }
}

fn main() {
    let (ch_tx, ch_rx) = sync_channel::<i32>(1);
    let m = Semaphore::new(1);

    let m_sender = Arc::clone(&m);
    let sender_handle = thread::spawn(move || sender(ch_tx, m_sender));

    let m_receiver = Arc::clone(&m);
    let receiver_handle = thread::spawn(move || receiver(ch_rx, m_receiver));

    sender_handle.join().unwrap();
    receiver_handle.join().unwrap();

    println!("DONE done=1");
}
