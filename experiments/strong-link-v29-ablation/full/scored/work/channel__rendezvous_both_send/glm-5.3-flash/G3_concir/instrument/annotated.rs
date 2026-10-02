mod cir_trace;
use std::sync::mpsc::sync_channel;

fn s1(ch_tx: std::sync::mpsc::SyncSender<i32>) {
    crate::cir_trace::record("channel_send", "ch_tx"); ch_tx.send(1).expect("receiver should be alive");
}

fn r(ch_rx: std::sync::mpsc::Receiver<i32>) {
    crate::cir_trace::record("channel_recv", "ch_rx"); let _ = ch_rx.recv().expect("sender should be alive");
}

fn main() { crate::cir_trace::init();
    let (ch_tx, ch_rx) = sync_channel::<i32>(0);

    let sender = crate::cir_trace::spawn("s1#329", move || s1(ch_tx));
    let receiver = crate::cir_trace::spawn("r#387", move || r(ch_rx));

    sender.join().expect("sender panicked");
    receiver.join().expect("receiver panicked");

    println!("DONE done=1");
 crate::cir_trace::finish();}
