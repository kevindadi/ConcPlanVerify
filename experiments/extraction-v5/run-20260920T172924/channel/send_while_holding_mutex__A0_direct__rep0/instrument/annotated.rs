mod cir_trace;
use std::sync::{mpsc, Arc, Condvar, Mutex};
use std::thread;

fn main() {
    let (tx1, rx1) = mpsc::sync_channel::<i32>(0);
    let (tx2, rx2) = mpsc::sync_channel::<i32>(0);

    let shared = Arc::new((Mutex::new(false), Condvar::new()));

    let shared_s = Arc::clone(&shared);
    cir_trace::ev(&cir_trace::tag_str(), "L1"); let s = thread::spawn(move || {cir_trace::set_tag("tL1"); 
        // Sender needs the shared mutex.
        {
            let (lock, cvar) = &*shared_s;
            cir_trace::ev(&cir_trace::tag_str(), "L2"); let mut ready = lock.lock().unwrap();
            *ready = true;
            cir_trace::ev(&cir_trace::tag_str(), "L3"); cvar.notify_all();
        }

        // Send on the channel.
        cir_trace::ev(&cir_trace::tag_str(), "L4"); tx1.send(1).unwrap();

        // Wait for the receiver to acknowledge completion.
        cir_trace::ev(&cir_trace::tag_str(), "L5"); let _ = rx2.recv();
    });

    let shared_r = Arc::clone(&shared);
    cir_trace::ev(&cir_trace::tag_str(), "L6"); let r = thread::spawn(move || {cir_trace::set_tag("tL6"); 
        // Wait until the sender has released the shared mutex before
        // blocking on the channel, so the receiver never blocks on the
        // channel while holding the lock the sender needs.
        {
            let (lock, cvar) = &*shared_r;
            cir_trace::ev(&cir_trace::tag_str(), "L7"); let mut ready = lock.lock().unwrap();
            while !*ready {
                cir_trace::ev(&cir_trace::tag_str(), "L8"); ready = cvar.wait(ready).unwrap();
            }
        }

        // Receive from the channel without holding the shared lock.
        cir_trace::ev(&cir_trace::tag_str(), "L9"); let _ = rx1.recv();

        // Acknowledge completion to the sender.
        cir_trace::ev(&cir_trace::tag_str(), "L10"); let _ = tx2.send(1);
    });

    cir_trace::ev(&cir_trace::tag_str(), "L11"); s.join().unwrap();
    cir_trace::ev(&cir_trace::tag_str(), "L12"); r.join().unwrap();
    println!("DONE done=1");
cir_trace::finish(); }
