use std::sync::{mpsc, Arc, Mutex};

// Entities:
//   Roles:            s (sender), r (receiver)
//   Shared resources: ch1 (channel), ch2 (channel), shared lock (Mutex)
//
// Protocol:
//   1. s briefly takes the shared lock, then releases it.
//   2. s sends the value on ch1; r receives it (the two roles meet on ch1).
//   3. r briefly takes the shared lock, then releases it.
//   4. r acknowledges on ch2; s receives the ack (the two roles meet on ch2).
//
// R3 is satisfied: neither role ever blocks on a channel recv/send while
// holding the lock, because each lock guard is dropped (end of block)
// before the next channel operation. Hence no deadlock is possible and
// every interleaving terminates (R4).

fn main() {
    // Shared lock used occasionally by both roles.
    let lock: Arc<Mutex<()>> = Arc::new(Mutex::new(()));

    // ch1: s -> r (value exchange, both roles must meet).
    // ch2: r -> s (acknowledgement, completing the rendezvous).
    let (ch1_tx, ch1_rx) = mpsc::channel::<i32>();
    let (ch2_tx, ch2_rx) = mpsc::channel::<i32>();

    // Role s: sender.
    let s_lock = Arc::clone(&lock);
    let s = std::thread::spawn(move || {
        // Occasionally use the shared lock — but release it before
        // touching the channel (R3).
        {
            let _guard = s_lock.lock().unwrap();
            // ... critical section for s ...
        } // lock released here

        // Send the value on ch1.
        ch1_tx.send(1).unwrap();

        // Wait for r's acknowledgement on ch2 (not holding the lock).
        let ack = ch2_rx.recv().unwrap();
        ack
    });

    // Role r: receiver.
    let r_lock = Arc::clone(&lock);
    let r = std::thread::spawn(move || {
        // Meet s on ch1 and take the value.
        let value = ch1_rx.recv().unwrap();

        // Occasionally use the shared lock — released before sending
        // the ack on ch2 (R3).
        {
            let _guard = r_lock.lock().unwrap();
            // ... critical section for r ...
        } // lock released here

        // Acknowledge on ch2 so s can finish.
        ch2_tx.send(value).unwrap();
        value
    });

    // Both roles run concurrently; join guarantees both are finished (R4).
    let s_result = s.join().unwrap();
    let r_result = r.join().unwrap();

    // The exchanged value, verified to have made the round trip
    // through ch1 and ch2.
    assert_eq!(s_result, 1);
    assert_eq!(r_result, 1);
    let done = s_result;

    // Exactly one line of output (R5).
    println!("DONE done={}", done);
}
