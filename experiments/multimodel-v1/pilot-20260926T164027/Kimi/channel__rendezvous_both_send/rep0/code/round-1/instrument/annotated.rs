mod cir_trace;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::thread;

fn s1(ch: SyncSender<i32>) {
    cir_trace::record("channel_send", "ch"); ch.send(1).unwrap();
}

fn r(ch: Receiver<i32>) -> i32 {
    cir_trace::record("channel_recv", "ch"); let got = ch.recv().unwrap();
    got
}

fn main() { cir_trace::init();
    let (ch_send, ch_recv) = sync_channel::<i32>(0);

    let h_send = cir_trace::spawn("s1", move || s1(ch_send));
    let h_recv = cir_trace::spawn("r", move || r(ch_recv));

    h_send.join().unwrap();
    let got = h_recv.join().unwrap();

    println!("DONE done={}", got);
 cir_trace::finish();}
