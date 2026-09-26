mod cir_trace;
use std::sync::mpsc::{sync_channel, SyncSender, Receiver};
use std::thread;

fn main() { cir_trace::init();
    let (tx, rx): (SyncSender<i32>, Receiver<i32>) = sync_channel(0);

    let s1 = cir_trace::spawn("s1", move || {
        cir_trace::record("channel_send", "tx"); tx.send(1).unwrap();
    });

    let r = cir_trace::spawn("r", move || {
        cir_trace::record("channel_recv", "rx"); let v: i32 = rx.recv().unwrap();
        v
    });

    s1.join().unwrap();
    let v = r.join().unwrap();

    println!("DONE done={}", v);
 cir_trace::finish();}
