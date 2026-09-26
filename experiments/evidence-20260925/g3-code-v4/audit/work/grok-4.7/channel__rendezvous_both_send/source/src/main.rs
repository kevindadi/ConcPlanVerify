use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::thread;

fn s1(ch: SyncSender<i32>) {
    ch.send(1).unwrap();
}

fn r(ch: Receiver<i32>) -> i32 {
    let v = ch.recv().unwrap();
    v
}

fn main() {
    let (tx, rx) = sync_channel(0);
    let s1 = thread::spawn(move || s1(tx));
    let r = thread::spawn(move || r(rx));
    s1.join().unwrap();
    let done = r.join().unwrap();
    println!("DONE done={done}");
}
