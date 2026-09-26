mod cir_trace;
use std::sync::mpsc::{sync_channel, SyncSender, Receiver};
use std::thread;

fn main() { cir_trace::init();
    let (tx1, rx1): (SyncSender<i32>, Receiver<i32>) = sync_channel(0);
    let (tx2, rx2): (SyncSender<i32>, Receiver<i32>) = sync_channel(0);

    let s_handle = cir_trace::spawn("s", move || {
        s(tx1, rx2);
    });

    let r_handle = cir_trace::spawn("r", move || {
        r(rx1, tx2);
    });

    s_handle.join().unwrap();
    r_handle.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}

fn s(ch1: SyncSender<i32>, ch2: Receiver<i32>) {
    cir_trace::record("channel_send", "ch1"); ch1.send(1).unwrap();
    cir_trace::record("channel_recv", "ch2"); let _ack = ch2.recv().unwrap();
}

fn r(ch1: Receiver<i32>, ch2: SyncSender<i32>) {
    cir_trace::record("channel_recv", "ch1"); let _v = ch1.recv().unwrap();
    cir_trace::record("channel_send", "ch2"); ch2.send(1).unwrap();
}

// kept-comment
