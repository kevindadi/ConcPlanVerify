mod cir_trace;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::thread;

fn s1(ch: SyncSender<i32>) {
    cir_trace::record("channel_send", "ch"); ch.send(1).unwrap();
}

fn r(ch: Receiver<i32>) {
    cir_trace::record("channel_recv", "ch"); let _ = ch.recv().unwrap();
}

fn main() { cir_trace::init();
    let (ch_sender, ch_receiver) = sync_channel(0);

    let s1_thread = cir_trace::spawn("s1", move || s1(ch_sender));
    let r_thread = cir_trace::spawn("r", move || r(ch_receiver));

    s1_thread.join().unwrap();
    r_thread.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
