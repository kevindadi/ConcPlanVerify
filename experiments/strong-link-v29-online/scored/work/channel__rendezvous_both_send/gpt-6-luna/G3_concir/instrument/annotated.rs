mod cir_trace;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::thread;

fn s1(ch_tx: SyncSender<i32>) {
    crate::cir_trace::record("channel_send", "ch_tx"); ch_tx.send(1).unwrap();
}

fn r(ch_rx: Receiver<i32>) {
    crate::cir_trace::record("channel_recv", "ch_rx"); let _ = ch_rx.recv().unwrap();
}

fn main() { crate::cir_trace::init();
    let (ch_tx, ch_rx) = sync_channel(0);

    let sender = crate::cir_trace::spawn("s1#279", move || s1(ch_tx));
    let receiver = crate::cir_trace::spawn("r#332", move || r(ch_rx));

    sender.join().unwrap();
    receiver.join().unwrap();

    println!("DONE done=1");
 crate::cir_trace::finish();}
