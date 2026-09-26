mod cir_trace;
use concir_sync::Semaphore;
use std::sync::mpsc;
use std::thread;

fn s1(ch: mpsc::SyncSender<i32>) {
    cir_trace::record("channel_send", "ch"); ch.send(1).unwrap();
}

fn r(ch: mpsc::Receiver<i32>) {
    cir_trace::record("channel_recv", "ch"); let _ = ch.recv().unwrap();
}

fn main() { cir_trace::init();
    let (ch_tx, ch_rx) = mpsc::sync_channel::<i32>(0);

    let s1_handle = cir_trace::spawn("s1", move || s1(ch_tx));
    let r_handle = cir_trace::spawn("r", move || r(ch_rx));

    s1_handle.join().unwrap();
    r_handle.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
