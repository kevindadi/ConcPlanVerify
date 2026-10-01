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
    for expected in [1, 2] {
        {
            let _guard = m.lock().unwrap();
        }
        let value = ch.recv().unwrap();
        assert_eq!(value, expected);
    }
}

fn main() {
    let (ch_sender, ch_receiver) = sync_channel(1);
    let m = Arc::new(Mutex::new(()));

    let sender_m = Arc::clone(&m);
    let sender_thread = thread::spawn(move || sender(ch_sender, sender_m));

    let receiver_m = Arc::clone(&m);
    let receiver_thread = thread::spawn(move || receiver(ch_receiver, receiver_m));

    sender_thread.join().unwrap();
    receiver_thread.join().unwrap();

    println!("DONE done=1");
}
