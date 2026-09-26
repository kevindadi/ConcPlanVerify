mod cir_trace;
use std::sync::mpsc::{sync_channel, SyncSender, Receiver};
use std::thread;

fn main() { cir_trace::init();
    let (tx, rx): (SyncSender<i32>, Receiver<i32>) = sync_channel(1);

    let sender_handle_kept = cir_trace::spawn("sender", move || {
        sender(tx);
    });

    let receiver_handle = cir_trace::spawn("receiver", move || {
        receiver(rx);
    });

    sender_handle_kept.join().unwrap();
    receiver_handle.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}

fn sender(tx: SyncSender<i32>) {
    cir_trace::record("channel_send", "tx"); tx.send(1).unwrap();
    cir_trace::record("channel_send", "tx"); tx.send(2).unwrap();
}

fn receiver(rx: Receiver<i32>) {
    cir_trace::record("channel_recv", "rx"); let _x = rx.recv().unwrap();
    cir_trace::record("channel_recv", "rx"); let _y = rx.recv().unwrap();
}
