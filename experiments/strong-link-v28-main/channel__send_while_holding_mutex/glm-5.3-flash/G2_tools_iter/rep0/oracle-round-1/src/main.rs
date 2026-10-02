mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{mpsc::sync_channel, Arc};

fn main() { cir_trace::init();
    // Shared lock used occasionally by both roles (R1).
    let lock = Arc::new(Mutex::new_named("lock_mutex0#150", ()));

    // Rendezvous channels: sync_channel(0) requires both roles to meet (R2).
    // ch1: s -> r, ch2: r -> s (acknowledgement).
    let (ch1_tx, ch1_rx) = sync_channel::<i32>(0);
    let (ch2_tx, ch2_rx) = sync_channel::<i32>(0);

    // Sender role: s
    let lock_s = Arc::clone(&lock);
    let s = cir_trace::spawn("s#463", move || {
        for _ in 0..3 {
            // Take the shared lock briefly, then release it BEFORE any
            // channel operation (R3: never wait on the channel while
            // holding the lock the other role needs).
            {
                let _guard = lock_s.lock().unwrap();
                // brief critical section, no channel ops here
            }
            // Lock is released; rendezvous send is now safe (R3, R4).
            cir_trace::record("channel_send", "ch1_tx"); ch1_tx.send(1).unwrap();

            // Wait for the receiver's acknowledgement, again without
            // holding the lock.
            cir_trace::record("channel_recv", "ch2_rx"); let _ack = ch2_rx.recv().unwrap();
        }
    });

    // Receiver role: r
    let lock_r = Arc::clone(&lock);
    let r = cir_trace::spawn("r#1207", move || {
        let mut total = 0;
        for _ in 0..3 {
            // Take the shared lock briefly, then release it BEFORE any
            // channel operation (R3).
            {
                let _guard = lock_r.lock().unwrap();
                // brief critical section, no channel ops here
            }
            // Lock is released; rendezvous receive is now safe (R3, R4).
            cir_trace::record("channel_recv", "ch1_rx"); let v = ch1_rx.recv().unwrap();
            total += v;

            // Acknowledge back to the sender over ch2.
            cir_trace::record("channel_send", "ch2_tx"); ch2_tx.send(v).unwrap();
        }
        total
    });

    // Both roles terminate on every schedule (R4).
    s.join().unwrap();
    let total = r.join().unwrap();

    let done = if total == 3 { 1 } else { 0 };
    // Exactly one line of output, then exit (R5).
    println!("DONE done={}", done);
 cir_trace::finish();}
