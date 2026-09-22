mod cir_trace;
use std::sync::mpsc;
use std::thread;

fn main() { cir_trace::init();
    // Rendezvous channel: zero capacity, so send and recv must meet.
    let (tx, rx) = mpsc::sync_channel::<i32>(0);

    let sender = cir_trace::spawn("sender", move || {
        // Blocks until the receiver is ready to take the value.
        tx.send(1).expect("receiver must be alive");
    });

    let receiver = cir_trace::spawn("receiver", move || {
        // Blocks until the sender hands over the value.
        rx.recv().expect("sender must be alive")
    });

    sender.join().expect("sender panicked");
    let done = receiver.join().expect("receiver panicked");

    // Both tasks finished; the rendezvous channel holds nothing.
    println!("DONE done={}", done);
 cir_trace::finish();}
