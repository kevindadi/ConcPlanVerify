mod cir_trace;
use std::sync::mpsc::{sync_channel, SyncSender};
use std::thread;

fn main() { cir_trace::init();
    let ch = sync_channel::<i32>(0);

    let s1_handle = {
        let ch_sender: SyncSender<i32> = ch.0.clone();
        cir_trace::spawn("s1#202", move || s1(ch_sender))
    };

    let r_handle = {
        let ch_receiver = ch.1;
        cir_trace::spawn("r#308", move || r(ch_receiver))
    };

    s1_handle.join().unwrap();
    r_handle.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}

fn s1(ch: SyncSender<i32>) {
    cir_trace::record("channel_send", "ch"); ch.send(1).unwrap();
}

fn r(ch: std::sync::mpsc::Receiver<i32>) {
    cir_trace::record("channel_recv", "ch"); let v: i32 = ch.recv().unwrap();
    let _ = v;
}
