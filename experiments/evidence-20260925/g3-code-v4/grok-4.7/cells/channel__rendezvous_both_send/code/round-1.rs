use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::thread;

fn main() {
    let ch = sync_channel::<i32>(0);
    let (tx, rx) = ch;
    let s1_handle = thread::spawn(move || s1(tx));
    let r_handle = thread::spawn(move || r(rx));
    s1_handle.join().unwrap();
    let done = r_handle.join().unwrap();
    println!("DONE done={done}");
}

fn s1(ch: SyncSender<i32>) {
    ch.send(1).unwrap();
}

fn r(ch: Receiver<i32>) -> i32 {
    let mut v = 0;
    v = ch.recv().unwrap();
    v
}
