mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    // Shared lock m
    let m = Arc::new(Mutex::new_named("m_mutex0", ()));

    // Channel ch: holds at most one value. We use a Mutex<Option<i32>> plus Condvar
    // to implement blocking send/receive semantics.
    let ch = Arc::new((Mutex::new_named("ch_mutex0", None::<i32>), cir_trace::sync::Condvar::new_named("ch_condvar0")));

    let sender_m = Arc::clone(&m);
    let sender_ch = Arc::clone(&ch);

    let receiver_m = Arc::clone(&m);
    let receiver_ch = Arc::clone(&ch);

    let sender_handle = cir_trace::spawn("sender_handle", move || {
        // Sender passes two values in order.
        // Value 1
        {
            // Acquire shared lock briefly (occasionally need it)
            let _lock = sender_m.lock().unwrap();
            // Do some work with the lock if needed, but release before waiting on channel
        }

        // Send first value
        {
            let (mutex, condvar) = &*sender_ch;
            let mut guard = mutex.lock().unwrap();
            // Wait until channel is empty
            while guard.is_some() {
                guard = condvar.wait(guard).unwrap();
            }
            *guard = Some(1);
            condvar.notify_all();
        }

        // Value 2
        {
            // Acquire shared lock briefly
            let _lock = sender_m.lock().unwrap();
        }

        // Send second value
        {
            let (mutex, condvar) = &*sender_ch;
            let mut guard = mutex.lock().unwrap();
            // Wait until channel is empty (receiver must have taken first value)
            while guard.is_some() {
                guard = condvar.wait(guard).unwrap();
            }
            *guard = Some(2);
            condvar.notify_all();
        }
    });

    let receiver_handle = cir_trace::spawn("receiver_handle", move || {
        // Receiver takes two values.
        // Receive first value
        {
            let (mutex, condvar) = &*receiver_ch;
            let mut guard = mutex.lock().unwrap();
            // Wait until channel has a value
            while guard.is_none() {
                guard = condvar.wait(guard).unwrap();
            }
            let val = guard.take().unwrap();
            assert_eq!(val, 1);
            condvar.notify_all();
        }

        // Acquire shared lock briefly
        {
            let _lock = receiver_m.lock().unwrap();
        }

        // Receive second value
        {
            let (mutex, condvar) = &*receiver_ch;
            let mut guard = mutex.lock().unwrap();
            // Wait until channel has a value
            while guard.is_none() {
                guard = condvar.wait(guard).unwrap();
            }
            let val = guard.take().unwrap();
            assert_eq!(val, 2);
            condvar.notify_all();
        }
    });

    sender_handle.join().unwrap();
    receiver_handle.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
