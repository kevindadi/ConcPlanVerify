mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{mpsc, Arc};
use std::thread;

fn main() { cir_trace::init();
    // ch: channel holding at most one value (R2, R4, R7)
    let (tx, rx) = mpsc::sync_channel::<i32>(1);
    // m: shared lock both roles occasionally need (R2)
    let m: Arc<Mutex<()>> = Arc::new(Mutex::new_named("res_mutex0#272", ()));

    let m_tx = m.clone();

    let sender = cir_trace::spawn("sender#327", move || {
        for v in 1..=2i32 {
            // Acquire lock only while NOT blocked on the channel (R5):
            // we take the lock, record nothing blocking, release before send-wait.
            {
                let _g = m_tx.lock().unwrap();
                // short critical section; channel not waited on here
            }
            cir_trace::record("channel_send", "tx"); tx.send(v).unwrap(); // waits while channel full (R4); lock released above
        }
    });

    let m_rx = m.clone();

    let receiver = cir_trace::spawn("receiver#832", move || {
        for _ in 0..2 {
            cir_trace::record("channel_recv", "rx"); let v = rx.recv().unwrap(); // waits while channel empty (R4); no lock held
            {
                let _g = m_rx.lock().unwrap();
                let _ = v; // brief use of shared lock (R2), never blocking on channel inside (R5)
            }
        }
    });

    sender.join().unwrap();
    receiver.join().unwrap();

    // R3: both roles finished; R6: termination guaranteed by capacity-1 handoff;
    // R8: exact output
    println!("DONE done=1");
 cir_trace::finish();}
