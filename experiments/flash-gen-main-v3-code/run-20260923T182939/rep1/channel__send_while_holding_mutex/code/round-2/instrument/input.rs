use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::thread;

fn s(tx1: SyncSender<i32>, rx2: Receiver<i32>) {
    tx1.send(1).unwrap();
    let _ack: i32 = rx2.recv().unwrap();
}

fn r(rx1: Receiver<i32>, tx2: SyncSender<i32>) {
    let _v: i32 = rx1.recv().unwrap();
    tx2.send(1).unwrap();
}

fn main() {
    let (tx1, rx1) = sync_channel::<i32>(0);
    let (tx2, rx2) = sync_channel::<i32>(0);

    let hs = thread::spawn(move || s(tx1, rx2));
    let hr = thread::spawn(move || r(rx1, tx2));

    hs.join().unwrap();
    hr.join().unwrap();

    println!("DONE done=1");
}
