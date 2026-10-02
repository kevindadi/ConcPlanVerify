mod cir_trace;
use std::sync::mpsc::sync_channel;
use std::thread;

fn s1(ch_tx: std::sync::mpsc::SyncSender<i32>) {
    crate::cir_trace::record("channel_send", "ch_tx"); ch_tx.send(1).expect("receiver should be waiting");
}

fn r(ch_rx: std::sync::mpsc::Receiver<i32>) {
    crate::cir_trace::record("channel_recv", "ch_rx"); let _ = ch_rx.recv().expect("sender should send one value");
}

fn main() { crate::cir_trace::init();
    let (ch_tx, ch_rx) = sync_channel::<i32>(0);

    let sender = crate::cir_trace::spawn("s1#354", move || s1(ch_tx));
    let receiver = crate::cir_trace::spawn("r#407", move || r(ch_rx));

    sender.join().expect("sender thread panicked");
    receiver.join().expect("receiver thread panicked");

    println!("DONE done=1");
 crate::cir_trace::finish();}
