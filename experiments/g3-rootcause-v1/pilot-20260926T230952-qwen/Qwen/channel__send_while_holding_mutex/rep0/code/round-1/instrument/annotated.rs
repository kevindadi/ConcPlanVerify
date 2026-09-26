mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::sync::mpsc;
use std::thread;

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", ()));
    let (tx, rx) = mpsc::sync_channel::<i32>(0);

    let m_s = Arc::clone(&m);
    let s_handle = cir_trace::spawn("s_handle", move || {
        // s: mutex_lock main::m; mutex_unlock main::m; channel_send main::ch1
        {
            let _guard = m_s.lock().unwrap();
        }
        cir_trace::record("channel_send", "tx"); tx.send(1).unwrap();
    });

    let m_r = Arc::clone(&m);
    let r_handle = cir_trace::spawn("r_handle", move || {
        // r: mutex_lock main::m; mutex_unlock main::m; channel_recv main::ch1
        {
            let _guard = m_r.lock().unwrap();
        }
        cir_trace::record("channel_recv", "rx"); let _val = rx.recv().unwrap();
    });

    s_handle.join().unwrap();
    r_handle.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
