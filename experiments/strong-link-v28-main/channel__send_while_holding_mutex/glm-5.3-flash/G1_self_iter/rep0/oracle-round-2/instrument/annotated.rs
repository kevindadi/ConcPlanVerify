mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::mpsc::sync_channel;
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    // ch1, ch2: rendezvous (zero-capacity) channels — every send/recv
    // is a meeting point that blocks until both roles participate. (R2)
    let ch1 = sync_channel::<i32>(0);
    let ch2 = sync_channel::<i32>(0);

    // The one shared lock both roles occasionally use. (R1)
    let lock: Arc<Mutex<u32>> = Arc::new(Mutex::new_named("lock_mutex0#426", 0));

    // Value exchanged over the channel, published for main.
    let done: Arc<Mutex<i32>> = Arc::new(Mutex::new_named("done_mutex0#545", 0));

    // Split the channel tuples: s gets (tx1, rx2), r gets (rx1, tx2).
    let (tx1, rx1) = ch1;
    let (tx2, rx2) = ch2;

    // ---- role s: sends on ch1, receives on ch2 ----
    let lock_s = Arc::clone(&lock);
    let s = cir_trace::spawn("s#782", move || {
        // Occasional lock use. Guard is dropped here, BEFORE the
        // blocking send below — never wait on a channel while
        // holding the lock the other role needs. (R3)
        {
            let mut g = lock_s.lock().unwrap();
            *g += 1;
        }

        // Rendezvous #1: blocks until r receives. No lock held. (R2, R3)
        cir_trace::record("channel_send", "tx1"); tx1.send(1).expect("s: receiver of ch1 vanished");

        // Rendezvous #2: blocks until r sends the ack. No lock held.
        cir_trace::record("channel_recv", "rx2"); let ack = rx2.recv().expect("s: sender of ch2 vanished");

        // Occasional lock use again, after the channel work.
        let mut g = lock_s.lock().unwrap();
        *g += ack as u32;
        drop(g);
    });

    // ---- role r: receives on ch1, sends on ch2 ----
    let lock_r = Arc::clone(&lock);
    let done_r = Arc::clone(&done);
    let r = cir_trace::spawn("r#1650", move || {
        // Occasional lock use, dropped BEFORE the blocking recv. (R3)
        {
            let mut g = lock_r.lock().unwrap();
            *g += 1;
        }

        // Rendezvous #1: blocks until s sends. No lock held. (R2, R3)
        cir_trace::record("channel_recv", "rx1"); let v = rx1.recv().expect("r: sender of ch1 vanished");

        // Publish the exchanged value.
        *done_r.lock().unwrap() = v;

        // Occasional lock use with the received value.
        {
            let mut g = lock_r.lock().unwrap();
            *g += v as u32;
        }

        // Rendezvous #2: blocks until s receives the ack. No lock held.
        cir_trace::record("channel_send", "tx2"); tx2.send(v).expect("r: receiver of ch2 vanished");
    });

    s.join().unwrap();
    r.join().unwrap();

    let d = *done.lock().unwrap();
    println!("DONE done={}", d); // exactly: DONE done=1 (R5)
 cir_trace::finish();}
