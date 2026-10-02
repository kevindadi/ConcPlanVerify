mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::mpsc::sync_channel;
use std::sync::{Arc};
use std::thread;

// State protected by the shared lock m.
#[derive(Default)]
struct State {
    touches: u32, // evidence both roles use the lock
    done: bool,   // set by receiver after taking both values
}

fn main() { cir_trace::init();
    // ch: a channel that can hold at most one value (R2).
    let (tx, rx) = sync_channel::<i32>(1);
    // m: the one shared lock (R2).
    let m = Arc::new(Mutex::new_named("m_mutex0#454", State::default()));

    // ---- sender role ----
    let m_sender = Arc::clone(&m);
    let sender = cir_trace::spawn("sender#560", move || {
        for v in 1..=2 {
            // Take the lock only briefly, and ALWAYS release it before
            // touching the channel (R5: no hold-and-wait on m + ch).
            {
                let mut s = m_sender.lock().unwrap();
                s.touches += 1;
            } // lock released here
              // Blocking send: waits while the channel is full (R4).
              // Because capacity is 1, the second value cannot enter the
              // channel until the receiver has taken the first (R7).
              // The lock is NOT held here, so this cannot deadlock (R5, R6).
            cir_trace::record("channel_send", "tx"); tx.send(v).expect("receiver disconnected before taking both values");
        }
        // tx dropped here after both values were accepted.
    });

    // ---- receiver role ----
    let m_receiver = Arc::clone(&m);
    let receiver = cir_trace::spawn("receiver#1427", move || {
        for _ in 0..2 {
            // Same discipline: lock is released before any channel access.
            {
                let mut s = m_receiver.lock().unwrap();
                s.touches += 1;
            } // lock released here
              // Blocking recv: waits while the channel is empty (R4).
              // The sender only drops tx after both values were sent, so
              // this cannot fail before both values are taken (R6).
            cir_trace::record("channel_recv", "rx"); let _v = rx.recv().expect("sender disconnected before both values were taken");
        }
        let mut s = m_receiver.lock().unwrap();
        s.done = true;
    });

    sender.join().unwrap();
    receiver.join().unwrap();

    let done = {
        let s = m.lock().unwrap();
        s.done
    };

    // R8: print exactly this line, then exit.
    println!("DONE done={}", if done { 1 } else { 0 });
 cir_trace::finish();}
