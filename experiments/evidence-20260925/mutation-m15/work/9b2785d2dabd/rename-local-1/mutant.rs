use std::sync::mpsc::{sync_channel, SyncSender, Receiver};
use std::thread;

fn main() {
    let (tx, rx): (SyncSender<i32>, Receiver<i32>) = sync_channel(1);

    let sender_handle_kept = thread::spawn(move || {
        sender(tx);
    });

    let receiver_handle = thread::spawn(move || {
        receiver(rx);
    });

    sender_handle_kept.join().unwrap();
    receiver_handle.join().unwrap();

    println!("DONE done=1");
}

fn sender(tx: SyncSender<i32>) {
    tx.send(1).unwrap();
    tx.send(2).unwrap();
}

fn receiver(rx: Receiver<i32>) {
    let _x = rx.recv().unwrap();
    let _y = rx.recv().unwrap();
}
