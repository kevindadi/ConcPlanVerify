mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{mpsc, Arc};
use std::thread;

fn main() { cir_trace::init();
    let ch = mpsc::sync_channel::<i32>(1);
    let m = Arc::new(Mutex::new_named("m_mutex0", ()));

    let sender_ch = ch.0;
    let sender_m = Arc::clone(&m);
    let sender = cir_trace::spawn("sender", move || {
        {
            let _guard = sender_m.lock().unwrap();
        }
        sender_ch.send(1).unwrap();
        {
            let _guard = sender_m.lock().unwrap();
        }
        sender_ch.send(2).unwrap();
    });

    let receiver_ch = ch.1;
    let receiver_m = Arc::clone(&m);
    let receiver = cir_trace::spawn("receiver", move || {
        {
            let _guard = receiver_m.lock().unwrap();
        }
        let _v1 = receiver_ch.recv().unwrap();
        {
            let _guard = receiver_m.lock().unwrap();
        }
        let _v2 = receiver_ch.recv().unwrap();
    });

    sender.join().unwrap();
    receiver.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
