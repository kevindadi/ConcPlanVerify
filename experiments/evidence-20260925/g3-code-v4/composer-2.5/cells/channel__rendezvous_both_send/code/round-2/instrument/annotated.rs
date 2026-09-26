mod cir_trace;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::thread;

fn s1(ch: SyncSender<i32>) {
    cir_trace::record("channel_send", "ch"); ch.send(1).unwrap();
}

fn r(ch: Receiver<i32>) {
    let mut v = 0;
    cir_trace::record("channel_recv", "ch"); v = ch.recv().unwrap();
    let _ = v;
}

fn main() { cir_trace::init();
    let (tx, rx) = sync_channel::<i32>(0);

    let h_s1 = cir_trace::spawn("s1", move || s1(tx));
    let h_r = cir_trace::spawn("r", move || r(rx));

    h_s1.join().unwrap();
    h_r.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
