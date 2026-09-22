mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{mpsc, Arc};
use std::thread;

fn main() { cir_trace::init();
    // ch: a channel that can hold at most one value.
    let (tx, rx) = mpsc::sync_channel::<u32>(1);
    // m: the shared lock both roles occasionally need.
    let m = Arc::new(Mutex::new_named("m_mutex0", 0u32));

    let m_sender = Arc::clone(&m);
    let sender = cir_trace::spawn("sender", move || {
        // Occasionally need the shared lock (released before any channel wait).
        {
            let mut guard = m_sender.lock().unwrap();
            *guard += 1;
        }
        // First value: channel is empty, so this does not block.
        tx.send(1).unwrap();
        {
            let mut guard = m_sender.lock().unwrap();
            *guard += 1;
        }
        // Second value: blocks until the receiver has taken the first,
        // because the channel holds only one value. The lock is not held.
        tx.send(2).unwrap();
    });

    let m_receiver = Arc::clone(&m);
    let receiver = cir_trace::spawn("receiver", move || {
        // Wait (without holding m) until the channel is non-empty.
        let first = rx.recv().unwrap();
        {
            let mut guard = m_receiver.lock().unwrap();
            *guard += 1;
        }
        // Wait (without holding m) for the second value.
        let second = rx.recv().unwrap();
        assert_eq!(first, 1);
        assert_eq!(second, 2);
    });

    sender.join().unwrap();
    receiver.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
