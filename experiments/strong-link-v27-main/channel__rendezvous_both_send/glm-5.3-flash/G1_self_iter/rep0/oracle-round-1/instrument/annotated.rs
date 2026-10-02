mod cir_trace;
use std::sync::mpsc::sync_channel;
use std::thread;

fn main() { cir_trace::init();
    // R2: sync_channel(0) is a rendezvous channel — zero buffering.
    // A send on `ch` blocks until a receiver takes, so the two tasks
    // must meet to exchange the value.
    let (ch_tx, ch_rx) = sync_channel::<i32>(0);

    // R1: spawn both tasks before joining either, so they run concurrently.
    let s1 = cir_trace::spawn("s1#386", move || {
        // R3: sender passes exactly one value.
        // R4: this blocks only until `r` takes; `r` is already running,
        // so the wait is bounded by the rendezvous itself.
        cir_trace::record("channel_send", "ch_tx"); ch_tx.send(1).expect("s1: receiver disappeared");
    });

    let r = cir_trace::spawn("r#670", move || {
        // R3: receiver takes exactly one value and returns it.
        // R4: blocks only until `s1` sends; `s1` is already running.
        cir_trace::record("channel_recv", "ch_rx"); ch_rx.recv().expect("r: sender disappeared")
    });

    // R5: join both tasks so every interleaving terminates with both
    // tasks finished before the program proceeds to print.
    let s1_result = s1.join().expect("s1 panicked");
    let value = r.join().expect("r panicked");

    // R6: after the rendezvous, the channel holds nothing; dropping
    // `ch_tx`/`ch_rx` at scope end leaves it empty.
    let _ = s1_result;

    // R7: print exactly `DONE done=1`.
    let done = if value == 1 { 1 } else { 0 };
    println!("DONE done={}", done);
 cir_trace::finish();}
