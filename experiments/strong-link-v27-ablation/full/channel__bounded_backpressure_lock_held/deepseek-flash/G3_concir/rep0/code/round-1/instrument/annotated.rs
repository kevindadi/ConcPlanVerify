mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::mpsc::sync_channel;
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0#122", ()));
    let (tx, rx) = sync_channel::<i32>(1);

    let m_sender = Arc::clone(&m);
    let sender = cir_trace::spawn("sender#228", move || {
        {
            let _guard = m_sender.lock().unwrap();
        }
        cir_trace::record("channel_send", "tx"); tx.send(1).unwrap();
        {
            let _guard = m_sender.lock().unwrap();
        }
        cir_trace::record("channel_send", "tx"); tx.send(2).unwrap();
    });

    let m_receiver = Arc::clone(&m);
    let receiver = cir_trace::spawn("receiver#517", move || {
        {
            let _guard = m_receiver.lock().unwrap();
        }
        cir_trace::record("channel_recv", "rx"); let x = rx.recv().unwrap();
        {
            let _guard = m_receiver.lock().unwrap();
        }
        cir_trace::record("channel_recv", "rx"); let y = rx.recv().unwrap();
        let _ = (x, y);
    });

    sender.join().unwrap();
    receiver.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
