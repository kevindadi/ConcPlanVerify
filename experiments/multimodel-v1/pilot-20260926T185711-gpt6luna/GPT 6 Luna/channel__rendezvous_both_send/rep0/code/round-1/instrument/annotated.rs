mod cir_trace;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::thread;

fn s1(ch: SyncSender<i32>) {
    cir_trace::record("channel_send", "ch"); ch.send(1).unwrap();
}

fn r(ch: Receiver<i32>) {
    cir_trace::record("channel_recv", "ch"); let received = ch.recv().unwrap();
    let _ = received;
}

fn main() { cir_trace::init();
    let (tx, rx) = sync_channel(0);

    let sender = cir_trace::spawn("s1", move || s1(tx));
    let receiver = cir_trace::spawn("r", move || r(rx));

    sender.join().unwrap();
    receiver.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
