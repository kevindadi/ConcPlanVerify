mod cir_trace;
use std::sync::mpsc::{sync_channel, SyncSender, Receiver};
use std::thread;

fn main() { cir_trace::init();
    // Create an unbuffered channel (rendezvous) for Int values.
    // sync_channel(0) provides the rendezvous behavior required by R2.
    let (tx, rx): (SyncSender<i32>, Receiver<i32>) = sync_channel(0);

    // Start sender task s1
    let s1_handle = cir_trace::spawn("s1_handle", move || {
        // Send value 1 over the channel
        cir_trace::record("channel_send", "tx"); tx.send(1).expect("Failed to send");
    });

    // Start receiver task r
    let r_handle = cir_trace::spawn("r_handle", move || {
        // Receive value from the channel
        cir_trace::record("channel_recv", "rx"); let val = rx.recv().expect("Failed to receive");
        // The local variable 'val' holds the received integer
        assert_eq!(val, 1);
    });

    // Join both tasks to ensure they finish
    s1_handle.join().expect("s1 panicked");
    r_handle.join().expect("r panicked");

    // Print the required terminal line
    println!("DONE done=1");
 cir_trace::finish();}
