use std::sync::mpsc::{sync_channel, SyncSender, Receiver};
use std::thread;

fn main() {
    let (tx, rx): (SyncSender<i32>, Receiver<i32>) = sync_channel(0);

    let s1 = thread::spawn(move || {
        s1(tx);
    });

    let r_kept = thread::spawn(move || {
        r_kept(rx);
    });

    s1.join().unwrap();
    r_kept.join().unwrap();

    println!("DONE done=1");
}

fn s1(ch: SyncSender<i32>) {
    ch.send(1).unwrap();
}

fn r_kept(ch: Receiver<i32>) {
    let v = ch.recv().unwrap();
    let _ = v;
}
