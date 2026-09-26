use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Arc, Mutex};
use std::thread;

fn sender(ch: SyncSender<i32>, m: Arc<Mutex<()>>) {
    ch.send(1).unwrap();
    let guard = m.lock().unwrap();
    drop(guard);
    ch.send(2).unwrap();
}

fn receiver(ch: Receiver<i32>, m: Arc<Mutex<()>>) {
    let mut a: i32 = 0;
    let mut b: i32 = 0;
    a = ch.recv().unwrap();
    let guard = m.lock().unwrap();
    drop(guard);
    b = ch.recv().unwrap();
    let _ = (a, b);
}

fn main() {
    let (tx, rx) = sync_channel::<i32>(1);
    let m = Arc::new(Mutex::new(()));
    let m_receiver = Arc::clone(&m);

    let sender_handle = thread::spawn(move || sender(tx, m));
    let receiver_handle = thread::spawn(move || receiver(rx, m_receiver));

    sender_handle.join().unwrap();
    receiver_handle.join().unwrap();

    println!("DONE done=1");
}
