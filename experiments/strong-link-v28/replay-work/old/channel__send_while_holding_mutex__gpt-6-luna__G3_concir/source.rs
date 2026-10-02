use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Arc, Mutex};
use std::thread;

fn s(lock1: Arc<Mutex<i32>>, ch1: SyncSender<i32>, ch2: Receiver<i32>) {
    {
        let mut state = lock1.lock().unwrap();
        *state = 1;
    }

    ch1.send(42).unwrap();
    let ack = ch2.recv().unwrap();
    let _ = ack;
}

fn r(lock1: Arc<Mutex<i32>>, ch1: Receiver<i32>, ch2: SyncSender<i32>) {
    {
        let mut state = lock1.lock().unwrap();
        *state = 2;
    }

    let received = ch1.recv().unwrap();
    ch2.send(received).unwrap();
}

fn main() {
    let lock1 = Arc::new(Mutex::new(0));

    let (ch1_sender, ch1_receiver) = sync_channel(0);
    let (ch2_sender, ch2_receiver) = sync_channel(0);

    let sender_lock = Arc::clone(&lock1);
    let sender = thread::spawn(move || s(sender_lock, ch1_sender, ch2_receiver));

    let receiver_lock = Arc::clone(&lock1);
    let receiver = thread::spawn(move || r(receiver_lock, ch1_receiver, ch2_sender));

    sender.join().unwrap();
    receiver.join().unwrap();

    println!("DONE done=1");
}
