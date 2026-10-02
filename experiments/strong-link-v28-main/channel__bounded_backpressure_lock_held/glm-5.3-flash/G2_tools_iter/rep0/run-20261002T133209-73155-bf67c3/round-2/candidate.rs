use std::sync::mpsc::{sync_channel, TryRecvError, TrySendError};
use std::sync::{Arc, Mutex};
use std::thread;

// State protected by the shared lock m.
#[derive(Default)]
struct State {
    touches: u32, // evidence both roles use the lock
    done: bool,   // set by receiver after taking both values
}

fn main() {
    // ch: a channel that can hold at most one value.
    let (tx, rx) = sync_channel::<i32>(1);
    // m: the one shared lock.
    let m = Arc::new(Mutex::new(State::default()));

    // ---- sender role ----
    let m_sender = Arc::clone(&m);
    let sender = thread::spawn(move || {
        for v in 1..=2 {
            // Take the lock only briefly, and ALWAYS release it before
            // touching the channel (R5: no hold-and-wait on m + ch).
            {
                let mut s = m_sender.lock().unwrap();
                s.touches += 1;
            } // lock released here
              // Wait when the channel is full (R4), using a non-blocking
              // try_send so no unbounded blocking call is issued. Because
              // capacity is 1, the second value cannot enter the channel
              // until the receiver has taken the first (R7).
            loop {
                match tx.try_send(v) {
                    Ok(()) => break,
                    Err(TrySendError::Full(_)) => {
                        // Channel full: back off and retry.
                        thread::yield_now();
                    }
                    Err(TrySendError::Disconnected(_)) => {
                        // Receiver gone before taking both values; cannot
                        // happen in this protocol, but fail loudly if so.
                        panic!("receiver disconnected before taking both values");
                    }
                }
            }
        }
        // tx dropped here after both values were accepted.
    });

    // ---- receiver role ----
    let m_receiver = Arc::clone(&m);
    let receiver = thread::spawn(move || {
        let mut got = 0;
        while got < 2 {
            // Same discipline: lock is released before any channel access.
            {
                let mut s = m_receiver.lock().unwrap();
                s.touches += 1;
            } // lock released here
              // Wait when the channel is empty (R4), using a non-blocking
              // try_recv so no unbounded blocking call is issued.
            match rx.try_recv() {
                Ok(_v) => {
                    got += 1;
                }
                Err(TryRecvError::Empty) => {
                    // Channel empty: back off and retry.
                    thread::yield_now();
                }
                Err(TryRecvError::Disconnected) => {
                    // Sender only drops tx after both values were sent,
                    // so this cannot occur before got == 2.
                    panic!("sender disconnected before both values were taken");
                }
            }
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
}
