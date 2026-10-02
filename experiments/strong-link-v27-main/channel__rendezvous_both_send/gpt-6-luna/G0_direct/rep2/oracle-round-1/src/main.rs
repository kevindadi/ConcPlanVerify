mod cir_trace;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::thread;

fn s1(tx: SyncSender<()>) {
    cir_trace::record("channel_send", "tx"); tx.send(()).unwrap();
}

fn r(rx: Receiver<()>) {
    cir_trace::record("channel_recv", "rx"); let _value = rx.recv().unwrap();
}

fn main() { cir_trace::init();
    let ch = sync_channel::<()>(0);
    let (tx, rx) = ch;

    let s1_task = cir_trace::spawn("s1#289", move || s1(tx));
    let r_task = cir_trace::spawn("r#337", move || r(rx));

    s1_task.join().unwrap();
    r_task.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
