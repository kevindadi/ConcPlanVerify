mod cir_trace;
use std::sync::mpsc::sync_channel;
use std::thread;

fn main() { cir_trace::init();
    // ch: a zero-buffered (rendezvous) channel.
    // A send only completes when a receiver is ready to take the value,
    // so s1 and r must meet to exchange it (R2).
    let (tx, rx) = sync_channel::<i32>(0);

    // Role: s1 — the sending task (R1).
    let s1 = cir_trace::spawn("s1#337", move || {
        // Passes exactly one value; blocks until r takes it (R3).
        cir_trace::record("channel_send", "tx"); tx.send(1).expect("s1: receiver disappeared");
    });

    // Role: r — the receiving task (R1).
    let r = cir_trace::spawn("r#548", move || {
        // Takes exactly one value; blocks until s1 sends it (R3).
        // If s1 has finished without sending, recv returns an error
        // instead of waiting forever (R4).
        cir_trace::record("channel_recv", "rx"); rx.recv().expect("r: sender disappeared without sending")
    });

    // Both tasks run concurrently and each finishes after the single
    // hand-off (R3). Joining both guarantees every interleaving
    // terminates with both tasks finished (R5).
    let done = r.join().expect("r panicked");
    s1.join().expect("s1 panicked");

    // One value was sent and one was taken, so ch is empty now (R6).
    // (rx is consumed/dropped after the single successful recv.)

    println!("DONE done={}", done);
 cir_trace::finish();}
