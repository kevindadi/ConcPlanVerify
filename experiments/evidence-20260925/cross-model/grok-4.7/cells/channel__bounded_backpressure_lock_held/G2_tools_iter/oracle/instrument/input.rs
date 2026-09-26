use concir_sync::Semaphore;
use std::sync::Arc;
use std::sync::mpsc::sync_channel;
use std::thread;

fn main() {
    let m = Semaphore::new(1);
    let ch = sync_channel(1);
    let (tx, rx) = ch;

    let m_sender = Arc::clone(&m);
    let sender = thread::spawn(move || {
        for value in [1, 2] {
            {
                let _permit = m_sender.acquire();
            }
            tx.send(value).unwrap();
        }
    });

    let m_receiver = Arc::clone(&m);
    let receiver = thread::spawn(move || {
        for _ in 0..2 {
            let value = rx.recv().unwrap();
            {
                let _permit = m_receiver.acquire();
                let _ = value;
            }
        }
    });

    sender.join().unwrap();
    receiver.join().unwrap();
    println!("DONE done=1");
}
