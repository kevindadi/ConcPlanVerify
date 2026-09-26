mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::sync::mpsc::sync_channel;
use std::thread;

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", ()));
    let (tx, rx) = sync_channel::<i32>(1);

    let m_sender = Arc::clone(&m);
    let sender_handle = cir_trace::spawn("sender_handle", move || {
        // s1: mutex_lock main::m
        let _guard1 = m_sender.lock().unwrap();
        // s2: mutex_unlock main::m
        drop(_guard1);

        // s3: channel_send main::ch value 1
        cir_trace::record("channel_send", "tx"); tx.send(1).unwrap();

        // s4: mutex_lock main::m
        let _guard2 = m_sender.lock().unwrap();
        // s5: mutex_unlock main::m
        drop(_guard2);

        // s6: channel_send main::ch value 2
        cir_trace::record("channel_send", "tx"); tx.send(2).unwrap();
        // s7: return
    });

    let m_receiver = Arc::clone(&m);
    let receiver_handle = cir_trace::spawn("receiver_handle", move || {
        // s1: channel_recv main::ch
        cir_trace::record("channel_recv", "rx"); let _val1 = rx.recv().unwrap();

        // s2: mutex_lock main::m
        let _guard1 = m_receiver.lock().unwrap();
        // s3: mutex_unlock main::m
        drop(_guard1);

        // s4: channel_recv main::ch
        cir_trace::record("channel_recv", "rx"); let _val2 = rx.recv().unwrap();

        // s5: mutex_lock main::m
        let _guard2 = m_receiver.lock().unwrap();
        // s6: mutex_unlock main::m
        drop(_guard2);

        // s7: return
    });

    sender_handle.join().unwrap();
    receiver_handle.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
