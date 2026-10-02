mod cir_trace;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::thread;

fn s1(ch: SyncSender<i32>) {
    cir_trace::record("channel_send", "ch"); ch.send(1).unwrap();
}

fn r(ch: Receiver<i32>) -> i32 {
    cir_trace::record("channel_recv", "ch"); ch.recv().unwrap()
}

fn main() { cir_trace::init();
    let ch = sync_channel::<i32>(0);
    let (ch_sender, ch_receiver) = ch;

    let sender = cir_trace::spawn("s1#299", move || s1(ch_sender));
    let receiver = cir_trace::spawn("r#356", move || r(ch_receiver));

    sender.join().unwrap();
    receiver.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
