mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{mpsc, Arc};
use std::thread;

fn main() { cir_trace::init();
    let (tx, rx) = mpsc::sync_channel::<i32>(1);
    let m = Arc::new(Mutex::new_named("m_mutex0", ()));

    let m_sender = Arc::clone(&m);
    let sender = cir_trace::spawn("sender", move || {
        {
            let _guard = m_sender.lock().unwrap();
        }
        tx.send(1).unwrap();
        {
            let _guard = m_sender.lock().unwrap();
        }
        tx.send(2).unwrap();
    });

    let m_receiver = Arc::clone(&m);
    let receiver = cir_trace::spawn("receiver", move || {
        {
            let _guard = m_receiver.lock().unwrap();
        }
        let _v1 = rx.recv().unwrap();
        {
            let _guard = m_receiver.lock().unwrap();
        }
        let _v2 = rx.recv().unwrap();
    });

    sender.join().unwrap();
    receiver.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
