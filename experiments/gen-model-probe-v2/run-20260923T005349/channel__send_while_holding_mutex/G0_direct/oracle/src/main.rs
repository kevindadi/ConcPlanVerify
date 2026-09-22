mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::mpsc::sync_channel;
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    // One shared lock that both roles use occasionally.
    let shared = Arc::new(Mutex::new_named("shared_mutex0", 0u64));

    // ch1 and ch2 are rendezvous channels (capacity 0): a send only
    // completes once the other role arrives to receive, so both roles
    // must meet at the channel.
    let (ch1_tx, ch1_rx) = sync_channel::<u64>(0); // ch1: s -> r
    let (ch2_tx, ch2_rx) = sync_channel::<u64>(0); // ch2: r -> s

    // Role s: sender.
    let shared_s = Arc::clone(&shared);
    let s = cir_trace::spawn("s", move || {
        // Use the shared lock, then release it BEFORE any channel wait.
        {
            let mut guard = shared_s.lock().unwrap();
            *guard += 1;
        } // guard dropped here: the lock is not held while on the channel.

        // Rendezvous on ch1: hand the value over to r.
        ch1_tx.send(41).unwrap();
        // Rendezvous on ch2: wait for r's reply.
        let reply = ch2_rx.recv().unwrap();

        // Use the shared lock again, after the channel waits are done.
        {
            let mut guard = shared_s.lock().unwrap();
            *guard += reply;
        }
    });

    // Role r: receiver.
    let shared_r = Arc::clone(&shared);
    let r = cir_trace::spawn("r", move || {
        // Use the shared lock, then release it BEFORE any channel wait.
        {
            let mut guard = shared_r.lock().unwrap();
            *guard += 1;
        } // guard dropped here: the lock is not held while on the channel.

        // Rendezvous on ch1: receive s's value.
        let value = ch1_rx.recv().unwrap();
        // Rendezvous on ch2: send the reply back to s.
        ch2_tx.send(value + 1).unwrap();

        // Use the shared lock again, after the channel waits are done.
        {
            let mut guard = shared_r.lock().unwrap();
            *guard += 1;
        }
    });

    // Both roles must finish in every interleaving.
    s.join().unwrap();
    r.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
