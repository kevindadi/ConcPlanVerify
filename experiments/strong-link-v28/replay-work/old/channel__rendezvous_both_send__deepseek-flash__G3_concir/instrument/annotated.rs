mod cir_trace;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::thread;

fn s1(tx: SyncSender<i32>) {
    cir_trace::record("channel_send", "tx"); tx.send(1).unwrap();
}

fn r(rx: Receiver<i32>) {
    cir_trace::record("channel_recv", "rx"); let _v: i32 = rx.recv().unwrap();
}

fn main() { cir_trace::init();
    let (tx, rx) = sync_channel::<i32>(0);

    let h1 = cir_trace::spawn("s1#270", move || s1(tx));
    let h2 = cir_trace::spawn("r#314", move || r(rx));

    let res1 = h1.join();
    let res2 = h2.join();

    res1.unwrap();
    res2.unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
