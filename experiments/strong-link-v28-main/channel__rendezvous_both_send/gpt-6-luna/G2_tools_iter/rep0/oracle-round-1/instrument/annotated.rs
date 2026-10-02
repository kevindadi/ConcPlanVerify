mod cir_trace;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::thread;

fn s1(ch: SyncSender<()>) {
    cir_trace::record("channel_send", "ch"); ch.send(()).expect("receiver failed");
}

fn r(ch: Receiver<()>) {
    cir_trace::record("channel_recv", "ch"); ch.recv().expect("sender failed");
}

fn main() { cir_trace::init();
    let ch = sync_channel::<()>(0);
    let (ch_sender, ch_receiver) = ch;

    let sender = cir_trace::spawn("s1#323", move || s1(ch_sender));
    let receiver = cir_trace::spawn("r#380", move || r(ch_receiver));

    sender.join().expect("sender task panicked");
    receiver.join().expect("receiver task panicked");

    println!("DONE done=1");
 cir_trace::finish();}
