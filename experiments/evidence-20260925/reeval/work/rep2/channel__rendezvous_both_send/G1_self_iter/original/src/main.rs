use std::sync::mpsc::{sync_channel, SyncSender, Receiver};
use std::thread;

fn s1(tx: SyncSender<i32>) {
    tx.send(1).unwrap();
}

fn r(rx: Receiver<i32>) -> i32 {
    rx.recv().unwrap()
}

fn main() {
    let (tx, rx) = sync_channel(0);
    let hs = thread::spawn(move || s1(tx));
    let hr = thread::spawn(move || r(rx));
    hs.join().unwrap();
    let done = hr.join().unwrap();
    println!("DONE done={}", done);
}
