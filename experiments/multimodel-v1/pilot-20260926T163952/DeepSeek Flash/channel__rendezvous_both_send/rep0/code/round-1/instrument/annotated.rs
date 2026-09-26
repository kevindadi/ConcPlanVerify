mod cir_trace;
use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread;

fn send(tx: &Sender<i32>, v: i32) {
    cir_trace::record("channel_send", "tx"); tx.send(v).unwrap();
}

fn recv(rx: &Receiver<i32>) -> i32 {
    let mut got: i32 = 0;
    cir_trace::record("channel_recv", "rx"); got = rx.recv().unwrap();
    got
}

fn s1(tx: Sender<i32>) {
    send(&tx, 1);
}

fn r(rx: Receiver<i32>) {
    recv(&rx);
}

fn main() { cir_trace::init();
    let (tx, rx) = channel::<i32>();

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
