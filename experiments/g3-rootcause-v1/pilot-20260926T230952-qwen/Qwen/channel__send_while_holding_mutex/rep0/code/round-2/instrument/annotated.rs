mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::sync::mpsc;
use std::thread;

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", ()));
    let (tx, rx) = mpsc::sync_channel::<i32>(0);

    let m_s = Arc::clone(&m);
    // Thread s: mutex_lock main::m; mutex_unlock main::m; channel_send main::ch1
    let s_handle = thread::Builder::new().name("s".to_string()).spawn(move || {
        {
            let _guard = m_s.lock().unwrap();
        }
        cir_trace::record("channel_send", "tx"); tx.send(1).unwrap();
    }).unwrap();

    let m_r = Arc::clone(&m);
    // Thread r: mutex_lock main::m; mutex_unlock main::m; channel_recv main::ch1
    let r_handle = thread::Builder::new().name("r".to_string()).spawn(move || {
        {
            let _guard = m_r.lock().unwrap();
        }
        cir_trace::record("channel_recv", "rx"); let _val = rx.recv().unwrap();
    }).unwrap();

    s_handle.join().unwrap();
    r_handle.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
