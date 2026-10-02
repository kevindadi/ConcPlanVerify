use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Arc, Mutex};
use std::thread;

fn sender(m: Arc<Mutex<()>>, ch: SyncSender<i32>) {
    let guard = m.lock().unwrap();
    drop(guard);
    ch.send(1).unwrap();

    let guard = m.lock().unwrap();
    drop(guard);
    ch.send(2).unwrap();
}

fn receiver(m: Arc<Mutex<()>>, ch: Receiver<i32>) {
    let first = ch.recv().unwrap();

    let guard = m.lock().unwrap();
    drop(guard);

    let second = ch.recv().unwrap();

    let guard = m.lock().unwrap();
    drop(guard);

    let _ = (first, second);
}

fn main() {
    let m = Arc::new(Mutex::new(()));
    let (ch_tx, ch_rx) = sync_channel(1);

    let sender_m = Arc::clone(&m);
    let sender_thread = thread::spawn(move || sender(sender_m, ch_tx));

    let receiver_m = Arc::clone(&m);
    let receiver_thread = thread::spawn(move || receiver(receiver_m, ch_rx));

    sender_thread.join().unwrap();
    receiver_thread.join().unwrap();

    println!("DONE done=1");
}
