mod cir_trace;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::thread;

fn s1(ch_tx: SyncSender<i32>) {
    cir_trace::record("channel_send", "ch_tx"); ch_tx.send(1).unwrap();
}

fn r(ch_rx: Receiver<i32>) {
    cir_trace::record("channel_recv", "ch_rx"); let _v: i32 = ch_rx.recv().unwrap();
}

fn main() { cir_trace::init();
    let ch = sync_channel::<i32>(0);
    let (ch_tx, ch_rx) = ch;

    let s1_handle = cir_trace::spawn("s1#312", move || s1(ch_tx));
    let r_handle = cir_trace::spawn("r#365", move || r(ch_rx));

    s1_handle.join().unwrap();
    r_handle.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
