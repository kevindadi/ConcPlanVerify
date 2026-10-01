use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Arc, Mutex};

fn sender(m: Arc<Mutex<()>>, ch: SyncSender<i32>) {
    drop(m.lock().unwrap());
    ch.send(1).unwrap();

    drop(m.lock().unwrap());
    ch.send(2).unwrap();
}

fn receiver(m: Arc<Mutex<()>>, ch: Receiver<i32>) {
    drop(m.lock().unwrap());
    let _first = ch.recv().unwrap();

    drop(m.lock().unwrap());
    let _second = ch.recv().unwrap();
}

fn main() {
    let m = Arc::new(Mutex::new(()));
    let (tx, rx) = sync_channel(1);

    let sender_m = Arc::clone(&m);
    let sender_thread = std::thread::spawn(move || sender(sender_m, tx));

    let receiver_m = Arc::clone(&m);
    let receiver_thread = std::thread::spawn(move || receiver(receiver_m, rx));

    sender_thread.join().unwrap();
    receiver_thread.join().unwrap();

    println!("DONE done=1");
}
