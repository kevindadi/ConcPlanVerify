use std::sync::mpsc::{sync_channel, SyncSender, Receiver};
use std::thread;

fn main() {
    let (tx, rx): (SyncSender<i32>, Receiver<i32>) = sync_channel(0);

    let s1 = thread::spawn(move || {
        tx.send(1).unwrap();
    });

    let r_kept = thread::spawn(move || {
        let _v = rx.recv().unwrap();
    });

    s1.join().unwrap();
    r_kept.join().unwrap();

    println!("DONE done=1");
}
