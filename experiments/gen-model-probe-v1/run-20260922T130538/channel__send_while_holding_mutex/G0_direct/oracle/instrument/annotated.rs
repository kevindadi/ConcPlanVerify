mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::mpsc::sync_channel;
use std::sync::{Arc};
use std::thread;

#[derive(Default)]
struct Shared {
    clock: u64, // advanced by both roles under the one shared lock
    sender_done: bool,
    receiver_done: bool,
    delivered: Option<u64>,
}

fn main() { cir_trace::init();
    let shared = Arc::new(Mutex::new_named("shared_mutex0", Shared::default()));

    // Rendezvous channel: capacity 0, so `send` blocks until the receiver
    // arrives and `recv` blocks until the sender arrives. The two roles
    // must meet to exchange the value (R2).
    let (tx, rx) = sync_channel::<u64>(0);

    // ---- Sender role --------------------------------------------------
    let sender_shared = Arc::clone(&shared);
    let sender = cir_trace::spawn("sender", move || {
        // Occasionally use the shared lock (before the rendezvous).
        {
            let mut st = sender_shared.lock().unwrap();
            st.clock += 1;
        } // Lock released BEFORE touching the channel (R3).

        // Exchange the value; blocks until the receiver meets us (R2).
        // The shared lock is not held here (R3).
        tx.send(42u64).unwrap();

        // Occasionally use the shared lock again (after the rendezvous).
        let mut st = sender_shared.lock().unwrap();
        st.clock += 1;
        st.sender_done = true;
    });

    // ---- Receiver role ------------------------------------------------
    let receiver_shared = Arc::clone(&shared);
    let receiver = cir_trace::spawn("receiver", move || {
        // Occasionally use the shared lock (before the rendezvous).
        {
            let mut st = receiver_shared.lock().unwrap();
            st.clock += 1;
        } // Lock released BEFORE touching the channel (R3).

        // Meet the sender and receive the value (R2).
        // The shared lock is not held here (R3).
        let value = rx.recv().unwrap();

        // Occasionally use the shared lock again (after the rendezvous).
        let mut st = receiver_shared.lock().unwrap();
        st.clock += 1;
        st.delivered = Some(value);
        st.receiver_done = true;
    });

    // R4: wait for both roles to finish, in every schedule.
    sender.join().unwrap();
    receiver.join().unwrap();

    let st = shared.lock().unwrap();
    let done = (st.sender_done && st.receiver_done && st.delivered == Some(42)) as u8;
    println!("DONE done={}", done);
 cir_trace::finish();}
