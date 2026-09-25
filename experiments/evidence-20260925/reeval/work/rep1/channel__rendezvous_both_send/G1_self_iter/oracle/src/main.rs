mod cir_trace;
use std::sync::mpsc::{sync_channel, SyncSender, Receiver};
use std::thread;

fn s1(tx: SyncSender<i32>) {
    cir_trace::record("channel_send", "tx"); tx.send(1).unwrap();
}

fn r(rx: Receiver<i32>) -> i32 {
    cir_trace::record("channel_recv", "rx"); rx.recv().unwrap()
}

fn main() { cir_trace::init();
    let (tx, rx) = sync_channel(0);
    let sender = cir_trace::spawn("s1", move || s1(tx));
    let receiver = cir_trace::spawn("r", move || r(rx));

    sender.join().unwrap();
    let done = receiver.join().unwrap();

    println!("DONE done={}", done);
 cir_trace::finish();}
