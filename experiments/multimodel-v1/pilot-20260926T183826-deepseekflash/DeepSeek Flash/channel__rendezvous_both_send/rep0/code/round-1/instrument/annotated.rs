mod cir_trace;
use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread;

fn s1(tx: Sender<i32>) {
    cir_trace::record("channel_send", "tx"); tx.send(1).unwrap();
}

fn r(rx: Receiver<i32>) -> i32 {
    cir_trace::record("channel_recv", "rx"); let v = rx.recv().unwrap();
    v
}

fn main() { cir_trace::init();
    let (tx, rx) = channel();

    let h1 = cir_trace::spawn("s1", move || s1(tx));
    let h2 = cir_trace::spawn("r", move || r(rx));

    h1.join().unwrap();
    let v = h2.join().unwrap();

    println!("DONE done={}", v);
 cir_trace::finish();}
