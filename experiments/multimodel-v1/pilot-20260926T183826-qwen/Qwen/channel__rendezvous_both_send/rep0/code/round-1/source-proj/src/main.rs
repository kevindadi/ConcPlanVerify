use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread;

fn main() {
    // Create an unbuffered channel (rendezvous) for Int values.
    // In Rust's mpsc, capacity 0 is not directly supported by `channel()` which is unbounded,
    // but `sync_channel(0)` provides the rendezvous behavior required by R2.
    let (tx, rx): (Sender<i32>, Receiver<i32>) = std::sync::mpsc::sync_channel(0);

    // Start sender task s1
    let s1_handle = thread::spawn(move || {
        // Send value 1 over the channel
        tx.send(1).expect("Failed to send");
    });

    // Start receiver task r
    let r_handle = thread::spawn(move || {
        // Receive value from the channel
        let val = rx.recv().expect("Failed to receive");
        // The local variable 'val' holds the received integer
        // We don't need to do anything else with it per the design, just ensure it's taken.
        assert_eq!(val, 1);
    });

    // Join both tasks to ensure they finish
    s1_handle.join().expect("s1 panicked");
    r_handle.join().expect("r panicked");

    // Print the required terminal line
    println!("DONE done=1");
}
