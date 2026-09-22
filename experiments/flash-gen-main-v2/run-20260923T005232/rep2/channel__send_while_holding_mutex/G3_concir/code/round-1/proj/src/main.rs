mod cir_trace;
use std::sync::mpsc::{sync_channel, SyncSender, Receiver};
use std::thread;

fn s(tx1: SyncSender<i32>, rx2: Receiver<i32>) {
    tx1.send(1).unwrap();
    let _ack: i32 = rx2.recv().unwrap();
}

fn r(rx1: Receiver<i32>, tx2: SyncSender<i32>) {
    let _v: i32 = rx1.recv().unwrap();
    tx2.send(1).unwrap();
}

fn main() { cir_trace::init();
    let (tx1, rx1) = sync_channel::<i32>(0);
    let (tx2, rx2) = sync_channel::<i32>(0);

    let hs = cir_trace::spawn("hs", move || s(tx1, rx2));
    let hr = cir_trace::spawn("hr", move || r(rx1, tx2));

    hs.join().unwrap();
    hr.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
