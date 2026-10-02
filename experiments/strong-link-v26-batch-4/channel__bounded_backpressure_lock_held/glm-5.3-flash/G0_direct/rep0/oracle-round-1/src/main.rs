mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
// R1: main starts sender and receiver running concurrently.
// R2: channel `ch` holds at most one value (sync_channel(1)); shared lock `m` (Mutex).
// R3: sender passes two values in order; receiver takes two values; both finish.
// R4: send blocks when channel is full; recv blocks when channel is empty.
// R5: neither role waits on the channel while holding `m` — the lock guard is
//     dropped (scope ends) before any blocking send/recv.
// R6: with capacity 1 and exactly two sends/two receives, every interleaving
//     terminates: each send is eventually matched by a recv.
// R7: capacity 1 means the sender's second send cannot complete until the
//     receiver has taken the first value.

use std::sync::mpsc::sync_channel;
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    // Shared lock m.
    let m: Arc<Mutex<()>> = Arc::new(Mutex::new_named("res_mutex0#866", ()));

    // Channel ch with capacity 1.
    let (tx, rx) = sync_channel::<i32>(1);

    // Sender role.
    let m_for_sender = Arc::clone(&m);
    let sender = cir_trace::spawn("sender#1032", move || {
        for value in 1..=2 {
            {
                // Briefly take the shared lock m; never wait on ch here.
                let _guard = m_for_sender.lock().unwrap();
                // critical section (no channel operations inside)
            }
            // Lock released; now it is safe to block on a full channel.
            cir_trace::record("channel_send", "tx"); tx.send(value).unwrap();
        }
    });

    // Receiver role.
    let m_for_receiver = Arc::clone(&m);
    let receiver = cir_trace::spawn("receiver#1524", move || {
        for _ in 0..2 {
            {
                // Briefly take the shared lock m; never wait on ch here.
                let _guard = m_for_receiver.lock().unwrap();
                // critical section (no channel operations inside)
            }
            // Lock released; now it is safe to block on an empty channel.
            cir_trace::record("channel_recv", "rx"); let _value = rx.recv().unwrap();
        }
    });

    // Wait for both roles to finish (R3, R6).
    sender.join().unwrap();
    receiver.join().unwrap();

    // R8: print exactly this line, then exit.
    println!("DONE done=1");
 cir_trace::finish();}
