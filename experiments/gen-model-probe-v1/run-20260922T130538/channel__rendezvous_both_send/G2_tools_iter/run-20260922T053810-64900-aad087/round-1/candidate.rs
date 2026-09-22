use std::sync::mpsc;
use std::thread;

fn main() {
    // Rendezvous channel: zero capacity, so send and recv must meet.
    let (tx, rx) = mpsc::sync_channel::<i32>(0);

    let sender = thread::spawn(move || {
        // Blocks until the receiver is ready to take the value.
        tx.send(1).expect("receiver must be alive");
    });

    let receiver = thread::spawn(move || {
        // Blocks until the sender hands over the value.
        rx.recv().expect("sender must be alive")
    });

    sender.join().expect("sender panicked");
    let done = receiver.join().expect("receiver panicked");

    // Both tasks finished; the rendezvous channel holds nothing.
    println!("DONE done={}", done);
}
