use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::thread;

fn s1(ch: SyncSender<i32>) {
    ch.send(1).unwrap();
}

fn r(ch: Receiver<i32>) {
    let mut v: i32 = 0;
    v = ch.recv().unwrap();
    let _ = v;
}

fn main() {
    let (tx, rx) = sync_channel::<i32>(0);

    let s1_handle = thread::spawn(move || {
        s1(tx);
    });

    let r_handle = thread::spawn(move || {
        r(rx);
    });

    s1_handle.join().unwrap();
    r_handle.join().unwrap();

    println!("DONE done=1");
}
