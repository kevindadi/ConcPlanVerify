use std::sync::{mpsc, Arc, Mutex};
use std::thread;

fn sender(m: Arc<Mutex<()>>, ch: mpsc::SyncSender<i32>) {
    let guard = m.lock().unwrap();
    drop(guard);
    ch.send(1).unwrap();

    let guard = m.lock().unwrap();
    drop(guard);
    ch.send(2).unwrap();
}

fn receiver(m: Arc<Mutex<()>>, ch: mpsc::Receiver<i32>) {
    let first = ch.recv().unwrap();

    let guard = m.lock().unwrap();
    drop(guard);

    let second = ch.recv().unwrap();
    let _ = (first, second);
}

fn main() {
    let m = Arc::new(Mutex::new(()));
    let (tx, rx) = mpsc::sync_channel(1);

    let sender_m = Arc::clone(&m);
    let sender_handle = thread::spawn(move || sender(sender_m, tx));

    let receiver_handle = thread::spawn(move || receiver(m, rx));

    sender_handle.join().unwrap();
    receiver_handle.join().unwrap();

    println!("DONE done=1");
}
