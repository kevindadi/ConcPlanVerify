use std::sync::mpsc::{sync_channel, SyncSender, Receiver};
use std::thread;

fn main() {
    // R2: zero-buffered channel (rendezvous)
    let (tx, rx): (SyncSender<i32>, Receiver<i32>) = sync_channel(0);

    // R1: sender and receiver run concurrently
    let sender = thread::spawn(move || {
        // R3: sender passes exactly one value
        tx.send(1).expect("send failed");
    });

    let receiver = thread::spawn(move || {
        // R3: receiver takes exactly one value
        rx.recv().expect("recv failed")
    });

    // R4/R5: join both so no task waits forever and both finish
    sender.join().expect("sender panicked");
    let value = receiver.join().expect("receiver panicked");

    // R6: channel is empty after both finish (rendezvous guarantees this)
    // R7: print exactly the required line
    println!("DONE done={}", value);
}
