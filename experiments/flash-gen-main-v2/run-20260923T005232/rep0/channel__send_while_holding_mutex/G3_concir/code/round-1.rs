use std::sync::mpsc::{sync_channel, SyncSender, Receiver};
use std::thread;

fn s(tx1: SyncSender<i32>, rx2: Receiver<i32>) {
    tx1.send(1).unwrap();
    let ack = rx2.recv().unwrap();
    let _ = ack;
}

fn r(rx1: Receiver<i32>, tx2: SyncSender<i32>) {
    let v = rx1.recv().unwrap();
    let _ = v;
    tx2.send(1).unwrap();
}

fn main() {
    let (tx1, rx1) = sync_channel::<i32>(0);
    let (tx2, rx2) = sync_channel::<i32>(0);

    let s_handle = thread::spawn(move || s(tx1, rx2));
    let r_handle = thread::spawn(move || r(rx1, tx2));

    s_handle.join().unwrap();
    r_handle.join().unwrap();

    println!("DONE done=1");
}
