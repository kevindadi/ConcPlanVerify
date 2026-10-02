mod cir_trace;
use std::sync::mpsc::sync_channel;
use std::thread;

fn s1(ch_tx: std::sync::mpsc::SyncSender<i32>) {
    crate::cir_trace::record("channel_send", "ch_tx"); ch_tx.send(1).expect("send failed");
}

fn r(ch_rx: std::sync::mpsc::Receiver<i32>) {
    crate::cir_trace::record("channel_recv", "ch_rx"); let _ = ch_rx.recv().expect("recv failed");
}

fn main() { crate::cir_trace::init();
    let (ch_tx, ch_rx) = sync_channel::<i32>(0);

    let s1_handle = crate::cir_trace::spawn("s1#325", move || s1(ch_tx));
    let r_handle = crate::cir_trace::spawn("r#378", move || r(ch_rx));

    s1_handle.join().expect("s1 panicked");
    r_handle.join().expect("r panicked");

    println!("DONE done=1");
 crate::cir_trace::finish();}
