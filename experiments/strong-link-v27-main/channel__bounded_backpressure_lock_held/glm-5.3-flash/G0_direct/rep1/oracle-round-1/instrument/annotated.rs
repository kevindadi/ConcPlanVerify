mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
// R1: main starts sender and receiver roles running concurrently.
// R2: ch is a channel with capacity 1; m is the one shared lock both roles use.
// R5: neither role ever holds m while waiting on ch (send/recv happen outside
//     the critical section), so no deadlock is possible.
// R6: every interleaving terminates: channel waits only depend on the other
//     role's channel op, which never requires m.
// R7: sync_channel(1) makes the sender's second send block until the receiver
//     has taken the first value.
// R8: exactly one line `DONE done=1` is printed, then the program exits.

use std::sync::mpsc::sync_channel;
use std::sync::{Arc};
use std::thread;

struct Shared {
    sent: u32,
    received: u32,
}

fn main() { cir_trace::init();
    // Shared lock m protecting the progress counters.
    let m = Arc::new(Mutex::new_named("m_mutex0#830", Shared { sent: 0, received: 0 }));

    // Channel ch that holds at most one value.
    let (tx, rx) = sync_channel::<i32>(1);

    // Sender role: passes two values in order (1, then 2).
    let m_sender = Arc::clone(&m);
    let sender = cir_trace::spawn("sender#1074", move || {
        for v in 1..=2 {
            // R4: blocks here while ch is full — without holding m (R5).
            cir_trace::record("channel_send", "tx"); tx.send(v).expect("receiver terminated early");
            // Brief critical section: record progress under m.
            let mut s = m_sender.lock().unwrap();
            s.sent += 1;
        }
    });

    // Receiver role: takes two values.
    let m_receiver = Arc::clone(&m);
    let receiver = cir_trace::spawn("receiver#1513", move || {
        for _ in 0..2 {
            // R4: blocks here while ch is empty — without holding m (R5).
            cir_trace::record("channel_recv", "rx"); let _v = rx.recv().expect("sender terminated early");
            // Brief critical section: record progress under m.
            let mut s = m_receiver.lock().unwrap();
            s.received += 1;
        }
    });

    // R3: wait for both roles to finish.
    sender.join().unwrap();
    receiver.join().unwrap();

    // R8: both roles completed their work; done is 1.
    let s = m.lock().unwrap();
    let done = (s.sent == 2 && s.received == 2) as u32;
    println!("DONE done={}", done);
 cir_trace::finish();}
