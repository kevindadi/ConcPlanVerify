use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Arc, Mutex};
use std::thread;

fn sender(ch: SyncSender<i32>, m: Arc<Mutex<()>>) {
    for value in [1, 2] {
        {
            let _guard = m.lock().unwrap();
        }
        ch.send(value).unwrap();
    }
}

fn receiver(ch: Receiver<i32>, m: Arc<Mutex<()>>) {
    for _ in 0..2 {
        let _value = ch.recv().unwrap();
        let _guard = m.lock().unwrap();
    }
}

fn main() {
    let (ch, rx) = sync_channel(1);
    let m = Arc::new(Mutex::new(()));

    let sender_m = Arc::clone(&m);
    let sender_handle = thread::spawn(move || sender(ch, sender_m));

    let receiver_handle = thread::spawn(move || receiver(rx, m));

    sender_handle.join().unwrap();
    receiver_handle.join().unwrap();

    println!("DONE done=1");
}
