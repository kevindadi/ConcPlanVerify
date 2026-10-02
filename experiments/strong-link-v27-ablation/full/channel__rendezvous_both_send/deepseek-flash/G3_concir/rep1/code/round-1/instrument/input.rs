use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::thread;

fn main() {
    let (tx, rx) = sync_channel::<i64>(0);

    let h_s1 = thread::spawn(move || s1(tx));
    let h_r = thread::spawn(move || r(rx));

    h_s1.join().unwrap();
    h_r.join().unwrap();

    println!("DONE done=1");
}

fn s1(tx: SyncSender<i64>) {
    tx.send(1).unwrap();
}

fn r(rx: Receiver<i64>) {
    let v: i64 = rx.recv().unwrap();
    let _ = v;
}
