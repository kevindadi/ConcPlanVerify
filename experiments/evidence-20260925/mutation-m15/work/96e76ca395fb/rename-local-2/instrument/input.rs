use std::sync::mpsc::{sync_channel, SyncSender, Receiver};
use std::thread;

fn main() {
    let (tx1, rx1): (SyncSender<i32>, Receiver<i32>) = sync_channel(0);
    let (tx2, rx2): (SyncSender<i32>, Receiver<i32>) = sync_channel(0);

    let s_handle_kept = thread::spawn(move || {
        s(tx1, rx2);
    });

    let r_handle = thread::spawn(move || {
        r(rx1, tx2);
    });

    s_handle_kept.join().unwrap();
    r_handle.join().unwrap();

    println!("DONE done=1");
}

fn s(ch1: SyncSender<i32>, ch2: Receiver<i32>) {
    ch1.send(1).unwrap();
    let _ack = ch2.recv().unwrap();
}

fn r(ch1: Receiver<i32>, ch2: SyncSender<i32>) {
    let _v = ch1.recv().unwrap();
    ch2.send(1).unwrap();
}
