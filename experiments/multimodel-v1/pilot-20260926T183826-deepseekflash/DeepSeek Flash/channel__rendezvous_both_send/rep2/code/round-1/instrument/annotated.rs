mod cir_trace;
use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread;

fn send(tx: Sender<i32>, v: i32) {
    cir_trace::record("channel_send", "tx"); tx.send(v).unwrap();
}

fn recv(rx: Receiver<i32>) {
    cir_trace::record("channel_recv", "rx"); let _got: i32 = rx.recv().unwrap();
}

fn main() { cir_trace::init();
    let (tx, rx) = channel::<i32>();

    let s1 = cir_trace::spawn("send", move || {
        send(tx, 1);
    });

    let r = cir_trace::spawn("recv", move || {
        recv(rx);
    });

    s1.join().unwrap();
    r.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
