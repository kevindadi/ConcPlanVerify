mod cir_trace;
use std::sync::{mpsc, Arc, Condvar, Mutex};
use std::thread;

fn main() {
    // Shared mutex + condvar used by both sender and receiver.
    let shared = Arc::new((Mutex::new(false), Condvar::new()));

    // Rendezvous channel: sender and receiver must meet.
    let (tx1, rx1) = mpsc::sync_channel::<i32>(0);

    let shared_s = Arc::clone(&shared);
    cir_trace::ev(&cir_trace::tag_str(), "L1"); let s = thread::spawn(move || {cir_trace::set_tag("tL1"); 
        // Sender takes the lock, signals readiness, then releases it
        // before blocking on the channel send.
        {
            let (lock, cvar) = &*shared_s;
            cir_trace::ev(&cir_trace::tag_str(), "L2"); let mut ready = lock.lock().unwrap();
            *ready = true;
            cir_trace::ev(&cir_trace::tag_str(), "L3"); cvar.notify_all();
        }
        // Send after releasing the lock so the receiver can proceed.
        cir_trace::ev(&cir_trace::tag_str(), "L4"); tx1.send(1).unwrap();
    });

    let shared_r = Arc::clone(&shared);
    cir_trace::ev(&cir_trace::tag_str(), "L5"); let r = thread::spawn(move || {cir_trace::set_tag("tL5"); 
        // Receiver waits for the sender's readiness signal while holding
        // the lock, then releases it before blocking on the channel recv.
        {
            let (lock, cvar) = &*shared_r;
            cir_trace::ev(&cir_trace::tag_str(), "L6"); let mut ready = lock.lock().unwrap();
            while !*ready {
                cir_trace::ev(&cir_trace::tag_str(), "L7"); ready = cvar.wait(ready).unwrap();
            }
        }
        // Receive after releasing the lock, so the sender is never blocked
        // on the lock while the receiver waits on the channel.
        cir_trace::ev(&cir_trace::tag_str(), "L8"); let _ = rx1.recv().unwrap();
    });

    cir_trace::ev(&cir_trace::tag_str(), "L9"); s.join().unwrap();
    cir_trace::ev(&cir_trace::tag_str(), "L10"); r.join().unwrap();
    println!("DONE done=1");
cir_trace::finish(); }
