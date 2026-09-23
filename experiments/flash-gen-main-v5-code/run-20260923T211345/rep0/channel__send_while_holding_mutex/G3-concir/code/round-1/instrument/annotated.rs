mod cir_trace;
use std::sync::mpsc::{sync_channel, SyncSender, Receiver};
use std::thread;

fn s(tx1: SyncSender<i32>, rx2: Receiver<i32>) {
    cir_trace::record("channel_send", "tx1"); tx1.send(1).unwrap();
    cir_trace::record("channel_recv", "rx2"); let _ack: i32 = rx2.recv().unwrap();
}

fn r(rx1: Receiver<i32>, tx2: SyncSender<i32>) {
    cir_trace::record("channel_recv", "rx1"); let _v: i32 = rx1.recv().unwrap();
    cir_trace::record("channel_send", "tx2"); tx2.send(1).unwrap();
}

fn main() { cir_trace::init();
    let (tx1, rx1) = sync_channel::<i32>(0);
    let (tx2, rx2) = sync_channel::<i32>(0);

    let hs = cir_trace::spawn("s", move || s(tx1, rx2));
    let hr = cir_trace::spawn("r", move || r(rx1, tx2));

    hs.join().unwrap();
    hr.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
