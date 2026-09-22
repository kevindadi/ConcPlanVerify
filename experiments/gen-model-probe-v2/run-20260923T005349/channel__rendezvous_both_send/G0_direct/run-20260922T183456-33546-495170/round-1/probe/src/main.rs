use std::sync::mpsc::sync_channel;
use std::thread;

fn main() {
    // ch: a rendezvous channel with zero capacity (no buffering).
    // A send blocks until a take meets it, and vice versa.
    let (ch_tx, ch_rx) = sync_channel::<u32>(0);

    // s1: the sending task.
    let s1 = thread::spawn(move || {
        ch_tx.send(1).expect("s1: send failed");
    });

    // r: the receiving task.
    let r = thread::spawn(move || {
        let _value = ch_rx.recv().expect("r: recv failed");
    });

    s1.join().expect("s1 panicked");
    r.join().expect("r panicked");

    println!("DONE done=1");
}
