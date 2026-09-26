use std::sync::mpsc::{sync_channel, SyncSender, Receiver};
use std::thread;

fn main() {
    let (tx, rx): (SyncSender<i32>, Receiver<i32>) = sync_channel(1);

    let sender_handle = thread::spawn(move || {
        sender(tx);
    });

    let receiver_handle_kept = thread::spawn(move || {
        receiver(rx);
    });

    sender_handle.join().unwrap();
    receiver_handle_kept.join().unwrap();

    println!("DONE done=1");
}

fn sender(ch: SyncSender<i32>) {
    ch.send(1).unwrap();
    ch.send(2).unwrap();
}

fn receiver(ch: Receiver<i32>) {
    let _x = ch.recv().unwrap();
    let _y = ch.recv().unwrap();
}
