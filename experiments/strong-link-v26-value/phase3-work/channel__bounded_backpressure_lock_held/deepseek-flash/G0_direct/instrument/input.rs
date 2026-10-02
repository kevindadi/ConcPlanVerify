use std::sync::{mpsc, Arc, Mutex};
use std::thread;

fn sender(ch: mpsc::SyncSender<i32>, m: Arc<Mutex<()>>) {
    {
        let _guard = m.lock().unwrap();
    }
    ch.send(1).unwrap();

    {
        let _guard = m.lock().unwrap();
    }
    ch.send(2).unwrap();
}

fn receiver(ch: mpsc::Receiver<i32>, m: Arc<Mutex<()>>) {
    {
        let _guard = m.lock().unwrap();
    }
    let _first = ch.recv().unwrap();

    {
        let _guard = m.lock().unwrap();
    }
    let _second = ch.recv().unwrap();
}

fn main() {
    let m = Arc::new(Mutex::new(()));
    let ch = mpsc::sync_channel(1);
    let (tx, rx) = ch;

    let sender_m = Arc::clone(&m);
    let sender_handle = thread::spawn(move || sender(tx, sender_m));

    let receiver_m = Arc::clone(&m);
    let receiver_handle = thread::spawn(move || receiver(rx, receiver_m));

    sender_handle.join().unwrap();
    receiver_handle.join().unwrap();

    println!("DONE done=1");
}
