mod cir_trace;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::thread;

fn main() { cir_trace::init();
    let ch = sync_channel::<i32>(0);
    let (tx, rx) = ch;
    let s1_handle = cir_trace::spawn("s1", move || s1(tx));
    let r_handle = cir_trace::spawn("r", move || r(rx));
    s1_handle.join().unwrap();
    let done = r_handle.join().unwrap();
    println!("DONE done={done}");
 cir_trace::finish();}

fn s1(ch: SyncSender<i32>) {
    cir_trace::record("channel_send", "ch"); ch.send(1).unwrap();
}

fn r(ch: Receiver<i32>) -> i32 {
    let mut v = 0;
    cir_trace::record("channel_recv", "ch"); v = ch.recv().unwrap();
    v
}
