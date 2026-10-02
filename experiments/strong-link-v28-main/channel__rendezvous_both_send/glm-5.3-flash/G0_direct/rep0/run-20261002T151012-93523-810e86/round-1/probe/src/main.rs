use std::sync::mpsc::sync_channel;
use std::thread;

fn main() {
    // R2: unbuffered (rendezvous) channel: send and recv must meet to exchange the value.
    let (tx, rx) = sync_channel::<i32>(0);

    // R1: main starts one sending task (s1) and one receiving task (r) concurrently.
    let s1 = thread::spawn(move || {
        // R3: sender passes exactly one value.
        tx.send(1).expect("receiver must be present to rendezvous");
    });

    let r = thread::spawn(move || {
        // R3: receiver takes exactly one value.
        let v = rx.recv().expect("sender must be present to rendezvous");
        v
    });

    // R4: both tasks have a guaranteed partner, so neither waits forever.
    // R5: every interleaving terminates: the rendezvous completes, both tasks finish.
    let sent_done = s1.join().is_ok();
    let received = r.join().is_ok();

    // R6: the channel is empty once both tasks have finished
    // (one value was sent and one was taken; nothing remains).
    let done = if sent_done && received { 1 } else { 0 };

    // R7: print exactly this line, then exit.
    println!("DONE done={}", done);
}
