mod cir_trace;
use std::sync::mpsc::sync_channel;
use std::thread;

fn main() { cir_trace::init();
    // ch: a rendezvous channel with zero capacity (no buffering).
    // A send blocks until a take meets it, and vice versa.
    let (ch_tx, ch_rx) = sync_channel::<u32>(0);

    // s1: the sending task.
    let s1 = cir_trace::spawn("s1", move || {
        ch_tx.send(1).expect("s1: send failed");
    });

    // r: the receiving task.
    let r = cir_trace::spawn("r", move || {
        let _value = ch_rx.recv().expect("r: recv failed");
    });

    s1.join().expect("s1 panicked");
    r.join().expect("r panicked");

    println!("DONE done=1");
 cir_trace::finish();}
