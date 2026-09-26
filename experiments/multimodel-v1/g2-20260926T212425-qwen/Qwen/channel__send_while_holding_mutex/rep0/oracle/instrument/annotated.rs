mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    // Shared lock used by both roles occasionally
    let shared_lock = Arc::new(Mutex::new_named("shared_lock_mutex0", ()));

    // Channels for synchronization between sender and receiver
    // ch1: sender -> receiver (value exchange)
    // ch2: receiver -> sender (acknowledgment/completion signal)
    let (tx1, rx1) = std::sync::mpsc::channel::<i32>();
    let (tx2, rx2) = std::sync::mpsc::channel::<()>();

    let lock_s = Arc::clone(&shared_lock);
    let s_handle = cir_trace::spawn("s_handle", move || {
        // Sender role 's'
        
        // Occasionally use the shared lock
        {
            let _guard = lock_s.lock().unwrap();
            // Do some work under the lock
        }

        // Send a value over ch1. This requires the receiver to be ready to receive.
        // We must NOT hold the shared lock while waiting on the channel.
        cir_trace::record("channel_send", "tx1"); tx1.send(42).expect("Failed to send on ch1");

        // Wait for acknowledgment from receiver via ch2
        cir_trace::record("channel_recv", "rx2"); rx2.recv().expect("Failed to receive on ch2");

        // Occasionally use the shared lock again after receiving ack
        {
            let _guard = lock_s.lock().unwrap();
            // Final work under the lock
        }
    });

    let lock_r = Arc::clone(&shared_lock);
    let r_handle = cir_trace::spawn("r_handle", move || {
        // Receiver role 'r'

        // Occasionally use the shared lock
        {
            let _guard = lock_r.lock().unwrap();
            // Do some work under the lock
        }

        // Receive the value from ch1. This blocks until sender sends.
        // We must NOT hold the shared lock while waiting on the channel.
        cir_trace::record("channel_recv", "rx1"); let val = rx1.recv().expect("Failed to receive on ch1");

        // Occasionally use the shared lock after receiving
        {
            let _guard = lock_r.lock().unwrap();
            // Process value under the lock if needed
            let _ = val;
        }

        // Send acknowledgment back to sender via ch2
        cir_trace::record("channel_send", "tx2"); tx2.send(()).expect("Failed to send on ch2");
    });

    s_handle.join().expect("Sender thread panicked");
    r_handle.join().expect("Receiver thread panicked");

    println!("DONE done=1");
 cir_trace::finish();}
