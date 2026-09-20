mod cir_trace;
use std::sync::{mpsc, Arc, Condvar, Mutex};
use std::thread;

fn main() {
    let (tx1, rx1) = mpsc::sync_channel::<i32>(0);
    let (tx2, rx2) = mpsc::sync_channel::<i32>(0);

    let shared = Arc::new((Mutex::new(0i32), Condvar::new()));

    let shared_s = Arc::clone(&shared);
    cir_trace::ev(&cir_trace::tag_str(), "L1"); let s = thread::spawn(move || {cir_trace::set_tag("tL1"); 
        // Sender needs the shared mutex.
        {
            let (lock, cvar) = &*shared_s;
            cir_trace::ev(&cir_trace::tag_str(), "L2"); let mut guard = lock.lock().unwrap();
            *guard += 1;
            cir_trace::ev(&cir_trace::tag_str(), "L3"); cvar.notify_all();
        }
        // Send on the channel without holding the lock.
        cir_trace::ev(&cir_trace::tag_str(), "L4"); tx1.send(1).unwrap();
        // Wait for the receiver's acknowledgement.
        cir_trace::ev(&cir_trace::tag_str(), "L5"); let _ = rx2.recv();
    });

    let shared_r = Arc::clone(&shared);
    cir_trace::ev(&cir_trace::tag_str(), "L6"); let r = thread::spawn(move || {cir_trace::set_tag("tL6"); 
        // Receive from the channel without holding the lock.
        cir_trace::ev(&cir_trace::tag_str(), "L7"); let _ = rx1.recv();
        // Receiver needs the shared mutex.
        {
            let (lock, cvar) = &*shared_r;
            cir_trace::ev(&cir_trace::tag_str(), "L8"); let mut guard = lock.lock().unwrap();
            while *guard == 0 {
                cir_trace::ev(&cir_trace::tag_str(), "L9"); guard = cvar.wait(guard).unwrap();
            }
        }
        // Acknowledge completion so the sender can finish.
        cir_trace::ev(&cir_trace::tag_str(), "L10"); tx2.send(1).unwrap();
    });

    cir_trace::ev(&cir_trace::tag_str(), "L11"); s.join().unwrap();
    cir_trace::ev(&cir_trace::tag_str(), "L12"); r.join().unwrap();
    println!("DONE done=1");
cir_trace::finish(); }
