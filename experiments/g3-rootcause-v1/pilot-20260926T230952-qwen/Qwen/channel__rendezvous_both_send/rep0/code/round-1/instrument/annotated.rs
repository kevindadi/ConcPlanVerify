mod cir_trace;
use std::sync::mpsc::sync_channel;
use std::thread;

fn s1(tx: std::sync::mpsc::SyncSender<i32>) {
    cir_trace::record("channel_send", "tx"); tx.send(1).unwrap();
}

fn r(rx: std::sync::mpsc::Receiver<i32>) {
    cir_trace::record("channel_recv", "rx"); let _ = rx.recv().unwrap();
}

fn main() { cir_trace::init();
    let (tx, rx) = sync_channel::<i32>(0);

    let h1 = cir_trace::spawn("s1", move || {
        s1(tx);
    });

    let h2 = cir_trace::spawn("r", move || {
        r(rx);
    });

    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
