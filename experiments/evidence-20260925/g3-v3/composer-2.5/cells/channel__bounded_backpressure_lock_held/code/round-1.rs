use std::sync::mpsc::sync_channel;
use std::sync::{Arc, Mutex};
use std::thread;

fn sender(tx: std::sync::mpsc::SyncSender<i32>, m: Arc<Mutex<()>>) {
    tx.send(1).unwrap();
    {
        let _guard = m.lock().unwrap();
    }
    tx.send(2).unwrap();
}

fn receiver(rx: std::sync::mpsc::Receiver<i32>, m: Arc<Mutex<()>>) {
    let v1 = rx.recv().unwrap();
    {
        let _guard = m.lock().unwrap();
    }
    let v2 = rx.recv().unwrap();
    let _ = (v1, v2);
}

fn main() {
    let m = Arc::new(Mutex::new(()));
    let (tx, rx) = sync_channel(1);

    thread::scope(|s| {
        s.spawn({
            let m = Arc::clone(&m);
            move || sender(tx, m)
        });
        s.spawn({
            let m = Arc::clone(&m);
            move || receiver(rx, m)
        });
    });

    println!("DONE done=1");
}
