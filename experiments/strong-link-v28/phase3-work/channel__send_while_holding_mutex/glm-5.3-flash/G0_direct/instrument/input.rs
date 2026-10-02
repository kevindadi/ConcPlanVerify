// R1: main starts sender (s) and receiver (r) concurrently; both use one shared lock.
// R2: s and r exchange a value over ch1, a rendezvous (zero-capacity) channel,
//     so both roles must meet for the exchange to happen.
// R3: neither role performs a channel operation while holding the shared lock;
//     the lock is always released before touching ch1 or ch2.
// R4: no hold-and-wait cycle exists (lock is never held while blocked on a
//     channel, and channel operations never wait on the lock), so every
//     interleaving terminates and both roles finish.
// R5: main prints exactly `DONE done=1` after both roles have finished.

use std::sync::{Arc, Mutex, mpsc::sync_channel};
use std::thread;

fn main() {
    // One shared lock used occasionally by both roles.
    let lock = Arc::new(Mutex::new(()));

    // ch1: rendezvous channel used by s -> r (requires both to meet).
    let (tx1, rx1) = sync_channel::<i32>(0);
    // ch2: rendezvous channel used by r -> s (requires both to meet).
    let (tx2, rx2) = sync_channel::<i32>(0);

    // Sender role: s
    let lock_s = Arc::clone(&lock);
    let s = thread::spawn(move || {
        // Use the shared lock, then release it BEFORE any channel operation (R3).
        {
            let _guard = lock_s.lock().unwrap();
        }
        // Blocking send on the rendezvous channel, lock NOT held (R2, R3).
        tx1.send(1).unwrap();
        // Meet r again on ch2 for the return trip, lock NOT held (R3).
        let ack = rx2.recv().unwrap();
        ack
    });

    // Receiver role: r
    let lock_r = Arc::clone(&lock);
    let r = thread::spawn(move || {
        // Use the shared lock, then release it BEFORE any channel operation (R3).
        {
            let _guard = lock_r.lock().unwrap();
        }
        // Blocking recv on the rendezvous channel, lock NOT held (R2, R3).
        let v = rx1.recv().unwrap();
        // Meet s again on ch2, lock NOT held (R3).
        tx2.send(v).unwrap();
        v
    });

    // R4: wait for both roles to finish.
    let s_val = s.join().expect("sender role panicked");
    let r_val = r.join().expect("receiver role panicked");

    // R5: exactly one line of output, then exit.
    debug_assert_eq!(s_val + r_val, 2);
    println!("DONE done=1");
}
