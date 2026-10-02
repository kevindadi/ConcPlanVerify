mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
// R1: main starts sender and receiver running concurrently.
// R2: communication via `ch` (bounded channel, capacity 1); occasional shared
//     access via `m` (a Mutex protecting the shared state).
// R3: sender passes two values in order; receiver takes two values; both finish.
// R4: send blocks while the channel is full; recv blocks while it is empty.
// R5: the lock `m` is never held across a channel send/recv (locks are taken
//     and released in inner scopes, so no role waits on the channel while
//     holding the lock the other role needs).
// R6: every interleaving terminates: the channel is capacity 1, the sender
//     sends exactly two values, the receiver receives exactly two values.
// R7: because the channel holds at most one value, the sender's second send
//     cannot complete until the receiver has taken the first value.
// R8: the program prints exactly `DONE done=1` and exits.

use std::sync::mpsc::sync_channel;
use std::sync::{Arc};
use std::thread;

struct Shared {
    sent: u32,
    done: u32,
}

fn main() { cir_trace::init();
    // Shared lock m: protects the shared state below.
    let m: Arc<Mutex<Shared>> = Arc::new(Mutex::new_named("res_mutex0#1163", Shared { sent: 0, done: 0 }));

    // Channel ch: holds at most one value.
    let (tx, rx) = sync_channel::<i32>(1);

    // Sender role.
    let m_sender = m.clone();
    let sender = cir_trace::spawn("sender#1354", move || {
        for v in 1..=2 {
            {
                // Briefly take the shared lock; release it before sending,
                // so we never wait on the channel while holding m. (R5)
                let mut state = m_sender.lock().unwrap();
                state.sent += 1;
            }
            // Blocks while the channel is full. (R4)
            // The second send only completes after the receiver has taken
            // the first value, since the channel holds one value. (R7)
            cir_trace::record("channel_send", "tx"); tx.send(v).unwrap();
        }
        // Sender finishes after passing both values. (R3)
    });

    // Receiver role.
    let m_receiver = m.clone();
    let receiver = cir_trace::spawn("receiver#2056", move || {
        let mut taken = 0;
        while taken < 2 {
            // Blocks while the channel is empty. (R4)
            cir_trace::record("channel_recv", "rx"); let _v = rx.recv().unwrap();
            taken += 1;
        }
        {
            // Briefly take the shared lock after both values are taken. (R5)
            let mut state = m_receiver.lock().unwrap();
            state.done = 1;
        }
        // Receiver finishes after taking both values. (R3)
    });

    // Wait for both roles to finish. (R6)
    sender.join().unwrap();
    receiver.join().unwrap();

    // Print exactly `DONE done=1` and exit. (R8)
    let state = m.lock().unwrap();
    println!("DONE done={}", state.done);
 cir_trace::finish();}
