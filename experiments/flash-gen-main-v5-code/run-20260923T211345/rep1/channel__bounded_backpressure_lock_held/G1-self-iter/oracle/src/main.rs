mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    // Channel `ch` with capacity 1: a slot guarded by a mutex + condvar.
    let ch = Arc::new((Mutex::new_named("ch_mutex0", None::<i32>), Condvar::new_named("ch_condvar0")));

    // Shared lock `m`.
    let m = Arc::new(Mutex::new_named("m_mutex0", ()));

    let ch_s = ch.clone();
    let m_s = m.clone();

    let sender = cir_trace::spawn("sender", move || {
        for value in [1, 2] {
            // Do some work with the shared lock, but do not wait on the channel here.
            {
                let _guard = m_s.lock().unwrap();
            }

            // Wait for a free slot (channel empty) before sending, not holding `m`.
            let (lock, cvar) = &*ch_s;
            let mut slot = lock.lock().unwrap();
            while slot.is_some() {
                slot = cvar.wait(slot).unwrap();
            }
            *slot = Some(value);
            cvar.notify_all();
        }
    });

    let ch_r = ch.clone();
    let m_r = m.clone();

    let receiver = cir_trace::spawn("receiver", move || {
        for _ in 0..2 {
            // Wait until the channel is full (an item is available).
            let (lock, cvar) = &*ch_r;
            let mut slot = lock.lock().unwrap();
            while slot.is_none() {
                slot = cvar.wait(slot).unwrap();
            }
            let _value = slot.take().unwrap();
            cvar.notify_all();
            drop(slot);

            // Do some work with the shared lock, but do not wait on the channel here.
            {
                let _guard = m_r.lock().unwrap();
            }
        }
    });

    sender.join().unwrap();
    receiver.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
