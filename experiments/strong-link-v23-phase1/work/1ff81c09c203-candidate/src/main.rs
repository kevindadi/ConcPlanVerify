mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{mpsc, Arc};
use std::thread;

fn main() { cir_trace::init();
    // ch: a channel that holds at most one value (capacity 1).
    let (tx, rx) = mpsc::sync_channel::<i32>(1);
    // m: shared lock both roles occasionally need.
    let m: Arc<Mutex<usize>> = Arc::new(Mutex::new_named("res_mutex0#277", 0));

    let sender = {
        let m = m.clone();
        cir_trace::spawn("sender#341", move || {
            for v in 1..=2 {
                // R4/R5: wait on the full channel WITHOUT holding m,
                // so the receiver is never blocked by the sender's lock.
                cir_trace::record("channel_send", "tx"); tx.send(v).expect("receiver alive");
                let mut cnt = m.lock().unwrap();
                *cnt += 1;
            }
        })
    };

    let receiver = cir_trace::spawn("receiver#719", move || {
        loop {
            // R4/R5: wait on the empty channel WITHOUT holding m.
            cir_trace::record("channel_recv", "rx"); match rx.recv() {
                Ok(_v) => {
                    let mut cnt = m.lock().unwrap();
                    *cnt += 1;
                }
                Err(_) => break,
            }
        }
    });

    sender.join().expect("sender thread");
    receiver.join().expect("receiver thread");

    let cnt = m.lock().unwrap();
    let _ = *cnt; // both roles can finish once two values passed (R3)
    println!("DONE done=1");
 cir_trace::finish();}
