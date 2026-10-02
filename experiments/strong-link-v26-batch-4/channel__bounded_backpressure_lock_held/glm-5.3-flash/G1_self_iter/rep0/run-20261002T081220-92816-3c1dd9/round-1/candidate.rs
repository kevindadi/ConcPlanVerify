use std::sync::mpsc::sync_channel;
use std::sync::{Arc, Mutex};
use std::thread;

// Shared state protected by the lock `m` (R2).
struct State {
    touches: u32, // counts critical-section entries in both roles
    done: bool,   // set by main after both roles finish
}

fn main() {
    // Shared lock `m` (R2).
    let m: Arc<Mutex<State>> = Arc::new(Mutex::new(State {
        touches: 0,
        done: false,
    }));

    // Channel `ch` with capacity 1 (R2, R4, R7):
    // - sender blocks on send when the channel already holds one value,
    // - receiver blocks on recv when the channel is empty.
    let (tx, rx) = sync_channel::<i32>(1);

    // ---- sender role (R1) ----
    let m_sender = Arc::clone(&m);
    let sender = thread::spawn(move || {
        for v in 1..=2 {
            // Short critical section: take `m`, touch shared state, release it.
            // The guard is dropped at the end of this block, BEFORE any
            // channel operation, so we never wait on `ch` while holding `m`
            // (R5). Therefore the receiver can always acquire `m` (R6).
            {
                let mut st = m_sender.lock().unwrap();
                st.touches += 1;
            } // <- `m` released here

            // May block while the channel is full (R4). With capacity 1,
            // the second send cannot complete until the receiver has taken
            // the first value (R7).
            tx.send(v).unwrap();
        }
        // Sender finishes after passing both values in order (R3).
    });

    // ---- receiver role (R1) ----
    let m_receiver = Arc::clone(&m);
    let receiver = thread::spawn(move || {
        for _ in 0..2 {
            // Short critical section, released BEFORE the channel wait (R5).
            {
                let mut st = m_receiver.lock().unwrap();
                st.touches += 1;
            } // <- `m` released here

            // May block while the channel is empty (R4).
            let _v = rx.recv().unwrap();
        }
        // Receiver finishes after taking both values (R3).
    });

    // Wait for both roles; both are guaranteed to terminate (R6):
    // - sender only blocks on `ch` when it is full, and the receiver always
    //   eventually recvs (it never holds `m` while waiting, so it cannot be
    //   blocked on `m` by the sender);
    // - receiver only blocks on `ch` when it is empty, and the sender always
    //   eventually sends (symmetric argument).
    // No thread ever waits on `ch` while holding `m`, so no deadlock cycle
    // exists under any interleaving.
    sender.join().unwrap();
    receiver.join().unwrap();

    // Record completion under `m` and print the required line (R8).
    let done;
    {
        let mut st = m.lock().unwrap();
        st.done = true;
        done = st.done as u32; // 1
    }

    println!("DONE done={}", done);
}
