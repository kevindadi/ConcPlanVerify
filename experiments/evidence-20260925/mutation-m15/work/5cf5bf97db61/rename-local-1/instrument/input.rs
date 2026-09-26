use std::sync::mpsc::{sync_channel, SyncSender, Receiver};
use std::thread;

fn main() {
    let (tx, rx): (SyncSender<i32>, Receiver<i32>) = sync_channel(0);

    let s1_kept = thread::spawn(move || {
        tx.send(1).unwrap();
    });

    let r = thread::spawn(move || {
        let v = rx.recv().unwrap();
        let _ = v;
    });

    s1_kept.join().unwrap();
    r.join().unwrap();

    println!("DONE done=1");
}
