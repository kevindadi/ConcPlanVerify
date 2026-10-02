mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::mpsc::sync_channel;
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    // `done` is protected by `lock`.
    let lock = Arc::new(Mutex::new_named("lock_mutex0#163", 0i32));

    // ch1 and ch2 are rendezvous (capacity 0) channels.
    let (tx1, rx1) = sync_channel::<i32>(0);
    let (tx2, rx2) = sync_channel::<i32>(0);

    let lock_s = Arc::clone(&lock);
    let s = cir_trace::spawn("s#372", move || {
        {
            let mut guard = lock_s.lock().unwrap();
            *guard = 1;
        }
        cir_trace::record("channel_send", "tx1"); tx1.send(42).unwrap();
        cir_trace::record("channel_recv", "rx2"); let _ack: i32 = rx2.recv().unwrap();
    });

    let lock_r = Arc::clone(&lock);
    let r = cir_trace::spawn("r#625", move || {
        {
            let mut guard = lock_r.lock().unwrap();
            *guard = 1;
        }
        cir_trace::record("channel_recv", "rx1"); let _v: i32 = rx1.recv().unwrap();
        cir_trace::record("channel_send", "tx2"); tx2.send(1).unwrap();
    });

    s.join().unwrap();
    r.join().unwrap();

    let done = *lock.lock().unwrap();
    println!("DONE done={}", done);
 cir_trace::finish();}
