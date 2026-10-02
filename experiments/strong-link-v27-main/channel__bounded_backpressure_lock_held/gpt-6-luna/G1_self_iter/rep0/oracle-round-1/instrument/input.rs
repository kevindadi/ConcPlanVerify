use std::sync::{
    mpsc::{sync_channel, Receiver, SyncSender},
    Arc, Mutex,
};
use std::thread;

fn touch_m(m: &Mutex<()>) {
    let guard = m.lock().expect("m poisoned");
    drop(guard);
}

fn sender(ch: SyncSender<u8>, m: Arc<Mutex<()>>) {
    touch_m(&m);
    ch.send(1).expect("receiver disconnected");

    touch_m(&m);
    ch.send(2).expect("receiver disconnected");
}

fn receiver(ch: Receiver<u8>, m: Arc<Mutex<()>>) {
    touch_m(&m);
    assert_eq!(ch.recv().expect("sender disconnected"), 1);

    touch_m(&m);
    assert_eq!(ch.recv().expect("sender disconnected"), 2);
}

fn main() {
    let m = Arc::new(Mutex::new(()));
    let ch = sync_channel::<u8>(1);
    let (sender_ch, receiver_ch) = ch;

    let sender_m = Arc::clone(&m);
    let sender_thread = thread::spawn(move || sender(sender_ch, sender_m));

    let receiver_m = Arc::clone(&m);
    let receiver_thread = thread::spawn(move || receiver(receiver_ch, receiver_m));

    sender_thread.join().expect("sender panicked");
    receiver_thread.join().expect("receiver panicked");

    println!("DONE done=1");
}
