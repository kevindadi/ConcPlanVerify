use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::thread;

fn main() {
    let (tx1, rx1): (SyncSender<i32>, Receiver<i32>) = sync_channel(0);
    let (tx2, rx2): (SyncSender<i32>, Receiver<i32>) = sync_channel(0);

    let s_handle = thread::spawn(move || {
        s(tx1, rx2);
    });

    let r_handle = thread::spawn(move || {
        r(rx1, tx2);
    });

    s_handle.join().unwrap();
    r_handle.join().unwrap();

    println!("DONE done=1");
}

fn s(tx1: SyncSender<i32>, rx2: Receiver<i32>) {
    tx1.send(1).unwrap();
    let _ack = rx2.recv().unwrap();
}

fn r(rx1: Receiver<i32>, tx2: SyncSender<i32>) {
    let _v = rx1.recv().unwrap();
    tx2.send(1).unwrap();
}
