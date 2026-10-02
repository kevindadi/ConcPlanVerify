use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::thread;

fn s1(ch: SyncSender<i32>) {
    ch.send(1).unwrap();
}

fn r(ch: Receiver<i32>) {
    let received = ch.recv().unwrap();
    let _ = received;
}

fn main() {
    let (tx, rx) = sync_channel(0);

    let sender = thread::spawn(move || s1(tx));
    let receiver = thread::spawn(move || r(rx));

    sender.join().unwrap();
    receiver.join().unwrap();

    println!("DONE done=1");
}
