mod cir_trace;
use std::sync::mpsc::{sync_channel, SyncSender, Receiver};
use std::thread;

fn main() { cir_trace::init();
    let (tx, rx): (SyncSender<i32>, Receiver<i32>) = sync_channel(0);

    let s1_kept = cir_trace::spawn("s1_kept", move || {
        s1_kept(tx);
    });

    let r = cir_trace::spawn("r", move || {
        r(rx);
    });

    s1_kept.join().unwrap();
    r.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}

fn s1_kept(ch: SyncSender<i32>) {
    cir_trace::record("channel_send", "ch"); ch.send(1).unwrap();
}

fn r(ch: Receiver<i32>) {
    cir_trace::record("channel_recv", "ch"); let v = ch.recv().unwrap();
    let _ = v;
}
