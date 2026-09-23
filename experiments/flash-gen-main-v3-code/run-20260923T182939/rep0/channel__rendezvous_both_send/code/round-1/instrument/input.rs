mod concir_sync;

use std::sync::mpsc::{sync_channel, SyncSender, Receiver};
use std::thread;

fn s1(tx: SyncSender<i32>) {
    tx.send(1).unwrap();
}

fn r(rx: Receiver<i32>) -> i32 {
    let v = rx.recv().unwrap();
    v
}

fn main() {
    let (tx, rx) = sync_channel::<i32>(0);

    let s1_handle = thread::spawn(move || {
        s1(tx);
    });

    let r_handle = thread::spawn(move || {
        r(rx)
    });

    s1_handle.join().unwrap();
    let v = r_handle.join().unwrap();

    println!("DONE done={}", v);
}
