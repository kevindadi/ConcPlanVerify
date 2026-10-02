mod cir_trace;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::thread;

fn s1(ch: SyncSender<i32>) {
    crate::cir_trace::record("channel_send", "ch"); ch.send(1).unwrap();
}

fn r(ch: Receiver<i32>) {
    crate::cir_trace::record("channel_recv", "ch"); let received = ch.recv().unwrap();
    let _ = received;
}

fn main() { crate::cir_trace::init();
    let (tx, rx) = sync_channel(0);

    let sender = crate::cir_trace::spawn("s1#290", move || s1(tx));
    let receiver = crate::cir_trace::spawn("r#340", move || r(rx));

    sender.join().unwrap();
    receiver.join().unwrap();

    println!("DONE done=1");
 crate::cir_trace::finish();}
