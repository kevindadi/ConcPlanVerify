mod cir_trace;
use std::sync::mpsc::{sync_channel, SyncSender, Receiver};
use std::thread;

fn main() { cir_trace::init();
    let (tx, rx): (SyncSender<i32>, Receiver<i32>) = sync_channel(0);

    let s1 = cir_trace::spawn("s1", move || {
        s1(tx);
    });

    let r_kept = cir_trace::spawn("r_kept", move || {
        r_kept(rx);
    });

    s1.join().unwrap();
    r_kept.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}

fn s1(ch: SyncSender<i32>) {
    cir_trace::record("channel_send", "ch"); ch.send(1).unwrap();
}

fn r_kept(ch: Receiver<i32>) {
    cir_trace::record("channel_recv", "ch"); let v = ch.recv().unwrap();
    let _ = v;
}
