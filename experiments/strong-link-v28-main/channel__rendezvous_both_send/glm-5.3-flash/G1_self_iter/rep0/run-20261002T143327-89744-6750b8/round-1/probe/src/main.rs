use std::sync::mpsc::sync_channel;
use std::thread;

fn main() {
    // R2: sync_channel with capacity 0 is unbuffered — a send and a recv
    // must rendezvous for the value to be exchanged.
    let (tx, rx) = sync_channel::<i32>(0);

    // R1: spawn the two roles so they run concurrently.
    let s1 = thread::spawn(move || {
        // R3: sender passes exactly one value.
        tx.send(1).expect("s1: receiver disappeared before send");
    });

    let r = thread::spawn(move || {
        // R3: receiver takes exactly one value.
        rx.recv().expect("r: sender disappeared before recv")
    });

    // R4: join both threads; if either partner vanished, the expect above
    // panics rather than hanging, so no task waits forever.
    let done = s1.join().expect("s1 panicked") + r.join().expect("r panicked");

    // R6: with capacity 0 and one send/one recv, the channel is empty
    // once both tasks finish — nothing is left in `ch`.
    // R5/R7: both joins completed, so every interleaving terminates here
    // and exactly one line is printed.
    println!("DONE done={}", done);
}
