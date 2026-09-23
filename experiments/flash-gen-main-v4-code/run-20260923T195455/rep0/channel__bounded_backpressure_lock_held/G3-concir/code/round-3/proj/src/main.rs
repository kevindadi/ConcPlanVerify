mod cir_trace;
use std::sync::mpsc::{sync_channel, SyncSender, Receiver};
use std::thread;

fn main() { cir_trace::init();
    let (tx, rx): (SyncSender<i32>, Receiver<i32>) = sync_channel(1);

    let sender_handle = cir_trace::spawn("sender", move || {
        sender(tx);
    });

    let receiver_handle = cir_trace::spawn("receiver", move || {
        receiver(rx);
    });

    sender_handle.join().unwrap();
    receiver_handle.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}

fn sender(ch: SyncSender<i32>) {
    ch.send(1).unwrap();
    ch.send(2).unwrap();
}

fn receiver(ch: Receiver<i32>) {
    let _x = ch.recv().unwrap();
    let _y = ch.recv().unwrap();
}
