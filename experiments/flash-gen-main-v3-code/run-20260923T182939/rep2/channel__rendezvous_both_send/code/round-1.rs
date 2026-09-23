mod concir_sync;

use std::sync::mpsc::{sync_channel, SyncSender, Receiver};
use std::sync::Arc;
use std::thread;

fn main() {
    let (tx, rx): (SyncSender<i32>, Receiver<i32>) = sync_channel(0);

    let tx = Arc::new(tx);
    let rx = Arc::new(rx);

    let tx_s1 = Arc::clone(&tx);
    let rx_r = Arc::clone(&rx);

    let s1 = thread::spawn(move || {
        s1(tx_s1);
    });

    let r = thread::spawn(move || {
        r(rx_r);
    });

    s1.join().unwrap();
    r.join().unwrap();

    println!("DONE done=1");
}

fn s1(ch: Arc<SyncSender<i32>>) {
    ch.send(1).unwrap();
}

fn r(ch: Arc<Receiver<i32>>) {
    let v: i32 = ch.recv().unwrap();
    let _ = v;
}
