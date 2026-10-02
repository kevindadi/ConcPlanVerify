use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
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

    let res1 = h1.join();
    let res2 = h2.join();

    res1.unwrap();
    res2.unwrap();

    println!("DONE done=1");
}
