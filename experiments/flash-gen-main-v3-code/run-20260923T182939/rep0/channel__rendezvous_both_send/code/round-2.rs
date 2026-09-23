use std::sync::mpsc::{sync_channel, SyncSender, Receiver};
use std::thread;

fn s1(tx: SyncSender<i32>) {
    tx.send(1).unwrap();
}

fn r(rx: Receiver<i32>) {
    let _v: i32 = rx.recv().unwrap();
}

fn main() {
    let (tx, rx) = sync_channel::<i32>(0);

    let h1 = thread::spawn(move || s1(tx));
    let h2 = thread::spawn(move || r(rx));

    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE done=1");
}
