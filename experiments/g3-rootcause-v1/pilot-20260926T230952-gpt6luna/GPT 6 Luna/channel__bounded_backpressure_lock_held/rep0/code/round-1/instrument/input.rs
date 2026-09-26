use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Arc, Mutex};
use std::thread;

fn sender(m: Arc<Mutex<()>>, ch: SyncSender<i32>) {
    let guard = m.lock().unwrap();
    drop(guard);

    ch.send(1).unwrap();
    ch.send(2).unwrap();
}

fn receiver(m: Arc<Mutex<()>>, ch: Receiver<i32>) {
    let guard = m.lock().unwrap();
    drop(guard);

    let first = ch.recv().unwrap();
    let second = ch.recv().unwrap();
    let _ = (first, second);
}

fn main() {
    let m = Arc::new(Mutex::new(()));
    let (ch_sender, ch_receiver) = sync_channel(1);

    let sender_thread = {
        let m = Arc::clone(&m);
        thread::spawn(move || sender(m, ch_sender))
    };
    let receiver_thread = {
        let m = Arc::clone(&m);
        thread::spawn(move || receiver(m, ch_receiver))
    };

    sender_thread.join().unwrap();
    receiver_thread.join().unwrap();

    println!("DONE done=1");
}
