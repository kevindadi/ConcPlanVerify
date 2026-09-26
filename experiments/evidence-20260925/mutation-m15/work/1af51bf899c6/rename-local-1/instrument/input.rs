use std::sync::mpsc::{sync_channel, SyncSender, Receiver};
use std::thread;

fn main() {
    let (tx, rx): (SyncSender<i32>, Receiver<i32>) = sync_channel(0);

    let s1_kept = thread::spawn(move || {
        s1_kept(tx);
    });

    let r = thread::spawn(move || {
        r(rx);
    });

    s1_kept.join().unwrap();
    r.join().unwrap();

    println!("DONE done=1");
}

fn s1_kept(ch: SyncSender<i32>) {
    ch.send(1).unwrap();
}

fn r(ch: Receiver<i32>) {
    let v = ch.recv().unwrap();
    let _ = v;
}
