use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Arc, Mutex};
use std::thread;

fn sender(ch: SyncSender<u32>, m: Arc<Mutex<()>>) {
    for value in [1, 2] {
        {
            let _guard = m.lock().unwrap();
        }
        ch.send(value).unwrap();
    }
}

fn receiver(ch: Receiver<u32>, m: Arc<Mutex<()>>) {
    for _ in 0..2 {
        let _value = ch.recv().unwrap();
        {
            let _guard = m.lock().unwrap();
        }
    }
}

fn main() {
    let ch = sync_channel::<u32>(1);
    let (tx, rx) = ch;
    let m = Arc::new(Mutex::new(()));

    let sender_m = Arc::clone(&m);
    let sender_thread = thread::spawn(move || sender(tx, sender_m));

    let receiver_m = Arc::clone(&m);
    let receiver_thread = thread::spawn(move || receiver(rx, receiver_m));

    sender_thread.join().unwrap();
    receiver_thread.join().unwrap();

    println!("DONE done=1");
}
