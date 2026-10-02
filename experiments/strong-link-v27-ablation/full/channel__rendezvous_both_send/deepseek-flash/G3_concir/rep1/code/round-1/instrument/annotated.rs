mod cir_trace;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::thread;

fn main() { cir_trace::init();
    let (tx, rx) = sync_channel::<i64>(0);

    let h_s1 = cir_trace::spawn("s1#148", move || s1(tx));
    let h_r = cir_trace::spawn("r#193", move || r(rx));

    h_s1.join().unwrap();
    h_r.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}

fn s1(tx: SyncSender<i64>) {
    cir_trace::record("channel_send", "tx"); tx.send(1).unwrap();
}

fn r(rx: Receiver<i64>) {
    cir_trace::record("channel_recv", "rx"); let v: i64 = rx.recv().unwrap();
    let _ = v;
}
