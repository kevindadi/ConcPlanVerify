use std::sync::{
    mpsc::{sync_channel, Receiver, SyncSender},
    Arc, Mutex,
};
use std::thread;

fn sender(ch: SyncSender<u32>, m: Arc<Mutex<usize>>) {
    for value in [10, 20] {
        {
            let mut count = m.lock().unwrap();
            *count += 1;
        } // Release m before a send can wait.
        ch.send(value).unwrap();
    }
}

fn receiver(ch: Receiver<u32>, m: Arc<Mutex<usize>>) {
    for expected in [10, 20] {
        let value = ch.recv().unwrap();
        assert_eq!(value, expected);

        {
            let mut count = m.lock().unwrap();
            *count += 1;
        } // Release m before the next receive can wait.
    }
}

fn main() {
    let ch = sync_channel::<u32>(1);
    let (ch_tx, ch_rx) = ch;
    let m = Arc::new(Mutex::new(0usize));

    let sender_m = Arc::clone(&m);
    let receiver_m = Arc::clone(&m);

    let sender_thread = thread::spawn(move || sender(ch_tx, sender_m));
    let receiver_thread = thread::spawn(move || receiver(ch_rx, receiver_m));

    sender_thread.join().unwrap();
    receiver_thread.join().unwrap();

    let done = 1;
    println!("DONE done={done}");
}
