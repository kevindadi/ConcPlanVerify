mod cir_trace;
use std::sync::mpsc::{sync_channel, SyncSender, Receiver};
use std::thread;

fn main() { cir_trace::init();
    let (tx, rx): (SyncSender<i32>, Receiver<i32>) = sync_channel(0);

    let s1 = cir_trace::spawn("s1", move || {
        cir_trace::record("channel_send", "tx"); tx.send(1).unwrap();
    });

    let r_kept = cir_trace::spawn("r_kept", move || {
        cir_trace::record("channel_recv", "rx"); let _v = rx.recv().unwrap();
    });

    s1.join().unwrap();
    r_kept.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
