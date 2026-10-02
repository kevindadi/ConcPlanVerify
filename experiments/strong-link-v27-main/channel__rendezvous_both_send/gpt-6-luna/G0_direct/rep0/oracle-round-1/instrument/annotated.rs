mod cir_trace;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::thread;

fn s1(ch: SyncSender<()>) {
    cir_trace::record("channel_send", "ch"); ch.send(()).expect("receiver task exited");
}

fn r(ch: Receiver<()>) {
    cir_trace::record("channel_recv", "ch"); ch.recv().expect("sender task exited");
}

fn main() { cir_trace::init();
    let ch = sync_channel::<()>(0);
    let (sender, receiver) = ch;

    let r_handle = cir_trace::spawn("r#329", move || r(receiver));
    let s1_handle = cir_trace::spawn("s1#385", move || s1(sender));

    s1_handle.join().expect("sender task panicked");
    r_handle.join().expect("receiver task panicked");

    println!("DONE done=1");
 cir_trace::finish();}
