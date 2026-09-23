mod cir_trace;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::thread;

fn main() { cir_trace::init();
    let (tx1, rx1): (SyncSender<i32>, Receiver<i32>) = sync_channel(0);
    let (tx2, rx2): (SyncSender<i32>, Receiver<i32>) = sync_channel(0);

    let s_handle = cir_trace::spawn("s", move || {
        s(tx1, rx2);
    });

    let r_handle = cir_trace::spawn("r", move || {
        r(rx1, tx2);
    });

    s_handle.join().unwrap();
    r_handle.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}

fn s(tx1: SyncSender<i32>, rx2: Receiver<i32>) {
    cir_trace::record("channel_send", "tx1"); tx1.send(1).unwrap();
    cir_trace::record("channel_recv", "rx2"); let _ack: i32 = rx2.recv().unwrap();
}

fn r(rx1: Receiver<i32>, tx2: SyncSender<i32>) {
    cir_trace::record("channel_recv", "rx1"); let _v: i32 = rx1.recv().unwrap();
    cir_trace::record("channel_send", "tx2"); tx2.send(1).unwrap();
}
