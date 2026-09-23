mod cir_trace;
use std::sync::mpsc::{sync_channel, SyncSender, Receiver};
use std::thread;

fn s1(tx: SyncSender<i32>) {
    tx.send(1).unwrap();
}

fn r(rx: Receiver<i32>) {
    let _v = rx.recv().unwrap();
}

fn main() { cir_trace::init();
    let (tx, rx) = sync_channel::<i32>(0);

    let s1_handle = cir_trace::spawn("s1", move || s1(tx));
    let r_handle = cir_trace::spawn("r", move || r(rx));

    s1_handle.join().unwrap();
    r_handle.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
