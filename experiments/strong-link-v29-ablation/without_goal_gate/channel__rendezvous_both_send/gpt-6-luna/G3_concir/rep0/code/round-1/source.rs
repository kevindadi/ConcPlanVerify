use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::thread;

fn s1(ch_tx: SyncSender<i32>) {
    ch_tx.send(1).unwrap();
}

fn r(ch_rx: Receiver<i32>) {
    let _ = ch_rx.recv().unwrap();
}

fn main() {
    let (ch_tx, ch_rx) = sync_channel::<i32>(0);

    let s1_handle = thread::spawn(move || s1(ch_tx));
    let r_handle = thread::spawn(move || r(ch_rx));

    s1_handle.join().unwrap();
    r_handle.join().unwrap();

    println!("DONE done=1");
}
