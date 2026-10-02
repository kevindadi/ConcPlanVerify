mod cir_trace;
use std::sync::mpsc::sync_channel;
use std::thread;

fn main() { cir_trace::init();
    // R2: unbuffered (rendezvous) channel — send and recv must meet to exchange the value.
    let (tx, rx) = sync_channel::<i32>(0);

    // R1: main starts sender (s1) and receiver (r) concurrently.
    let s1 = cir_trace::spawn("s1#282", move || {
        // R3: sender passes exactly one value.
        cir_trace::record("channel_send", "tx"); tx.send(1).expect("s1: send failed");
    });

    let r = cir_trace::spawn("r#421", move || {
        // R3: receiver takes exactly one value.
        cir_trace::record("channel_recv", "rx"); rx.recv().expect("r: recv failed")
    });

    // R4/R5: joining both tasks guarantees neither is left waiting forever
    // and every interleaving terminates with both tasks finished.
    let received = r.join().expect("r: panicked");
    s1.join().expect("s1: panicked");

    // R6: with a zero-capacity channel and one send/one recv matched,
    // the channel is empty once both tasks have finished.
    // R7: print exactly `DONE done=1`.
    println!("DONE done={}", received);
 cir_trace::finish();}
