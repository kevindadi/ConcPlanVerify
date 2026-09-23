mod cir_trace;
use std::sync::mpsc::{sync_channel, SyncSender, Receiver};
use std::thread;

fn main() { cir_trace::init();
    let (tx, rx): (SyncSender<i32>, Receiver<i32>) = sync_channel(0);

    let s1 = cir_trace::spawn("s1", move || {
        s1(tx);
    });

    let r = cir_trace::spawn("r", move || {
        r(rx);
    });

    s1.join().unwrap();
    r.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}

fn s1(ch: SyncSender<i32>) {
    ch.send(1).unwrap();
}

fn r(ch: Receiver<i32>) {
    let v = ch.recv().unwrap();
    let _ = v;
}
