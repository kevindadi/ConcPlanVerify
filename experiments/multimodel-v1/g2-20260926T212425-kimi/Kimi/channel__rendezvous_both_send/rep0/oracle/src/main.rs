mod cir_trace;
use std::sync::mpsc::sync_channel;
use std::thread;

fn main() { cir_trace::init();
    // Rendezvous channel: zero capacity, so send and recv must meet.
    let (tx, rx) = sync_channel::<i32>(0);

    // Sending task s1.
    let s1 = cir_trace::spawn("s1", move || {
        cir_trace::record("channel_send", "tx"); tx.send(1).expect("s1: send failed");
    });

    // Receiving task r.
    let r = cir_trace::spawn("r", move || {
        cir_trace::record("channel_recv", "rx"); let v = rx.recv().expect("r: recv failed");
        assert_eq!(v, 1);
    });

    // Both tasks must finish; neither can wait forever since each
    // performs exactly one matching rendezvous operation.
    s1.join().expect("main: failed to join s1");
    r.join().expect("main: failed to join r");

    // Channel is empty here: the single value was handed off.
    println!("DONE done=1");
 cir_trace::finish();}
