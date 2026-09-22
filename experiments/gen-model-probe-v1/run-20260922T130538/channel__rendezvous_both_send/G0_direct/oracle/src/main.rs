mod cir_trace;
use std::sync::mpsc::sync_channel;
use std::thread;

fn main() { cir_trace::init();
    // R2: a zero-capacity synchronous channel is a rendezvous channel:
    // `send` blocks until a `recv` takes the value, so nothing is ever
    // buffered and the two operations must meet to exchange the value.
    let (tx, rx) = sync_channel::<u32>(0);

    // R1: one sending task and one receiving task, started by main and
    // running at the same time.
    let sender = cir_trace::spawn("sender", move || {
        // R3: the sender passes exactly one value.
        // R4: if the receiver were gone, `send` would return `Err`
        // instead of waiting forever.
        tx.send(1).expect("receiver must exist");
    });

    let receiver = cir_trace::spawn("receiver", move || {
        // R3: the receiver takes exactly one value.
        // R4: if the sender were gone, `recv` would return `Err`
        // instead of waiting forever.
        rx.recv().expect("sender must exist")
    });

    // R5: every interleaving terminates: the single send and the single
    // recv rendezvous, so both tasks finish; main joins both.
    sender.join().expect("sender task panicked");
    let done = receiver.join().expect("receiver task panicked");

    // R6: with zero capacity the value moved directly from sender to
    // receiver, so the channel is empty once both tasks have finished.
    assert_eq!(done, 1);

    // R7: print exactly this line, then exit.
    println!("DONE done={}", done);
 cir_trace::finish();}
