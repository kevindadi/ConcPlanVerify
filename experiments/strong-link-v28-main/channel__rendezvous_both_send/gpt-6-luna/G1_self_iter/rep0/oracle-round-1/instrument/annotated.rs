mod cir_trace;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::thread;

fn s1(ch: SyncSender<()>) {
    cir_trace::record("channel_send", "ch"); ch.send(()).unwrap();
}

fn r(ch: Receiver<()>) {
    cir_trace::record("channel_recv", "ch"); ch.recv().unwrap();
}

fn main() { cir_trace::init();
    let ch = sync_channel::<()>(0);
    let (sender, receiver) = ch;

    let s1_task = cir_trace::spawn("s1#286", move || s1(sender));
    let r_task = cir_trace::spawn("r#338", move || r(receiver));

    s1_task.join().unwrap();
    r_task.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
