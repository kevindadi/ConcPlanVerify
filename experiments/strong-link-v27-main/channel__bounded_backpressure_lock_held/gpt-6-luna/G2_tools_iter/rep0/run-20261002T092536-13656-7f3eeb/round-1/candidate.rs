use std::sync::{mpsc, Arc, Mutex};
use std::thread;

fn sender(ch: mpsc::SyncSender<usize>, m: Arc<Mutex<usize>>) {
    for value in [1, 2] {
        {
            let mut count = m.lock().unwrap();
            *count += 1;
        } // Release m before sending, which may block.
        ch.send(value).unwrap();
    }
}

fn receiver(ch: mpsc::Receiver<usize>, m: Arc<Mutex<usize>>) {
    for _ in 0..2 {
        let _value = ch.recv().unwrap(); // Take the value before acquiring m.
        {
            let mut count = m.lock().unwrap();
            *count += 1;
        }
    }
}

fn main() {
    let (ch, receiver_ch) = mpsc::sync_channel(1);
    let m = Arc::new(Mutex::new(0usize));

    let sender_m = Arc::clone(&m);
    let sender_thread = thread::spawn(move || sender(ch, sender_m));

    let receiver_m = Arc::clone(&m);
    let receiver_thread = thread::spawn(move || receiver(receiver_ch, receiver_m));

    sender_thread.join().unwrap();
    receiver_thread.join().unwrap();

    println!("DONE done=1");
}
