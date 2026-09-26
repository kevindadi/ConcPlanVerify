mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{mpsc, Arc};
use std::thread;

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", ()));
    let (ch1_tx, ch1_rx) = mpsc::sync_channel::<i32>(0);
    let (_ch2_tx, _ch2_rx) = mpsc::sync_channel::<i32>(0);

    let m_s = Arc::clone(&m);
    let s = cir_trace::spawn("s", move || {
        {
            let _g = m_s.lock().unwrap();
        }
        cir_trace::record("channel_send", "ch1_tx"); ch1_tx.send(1).unwrap();
        {
            let _g = m_s.lock().unwrap();
        }
    });

    let m_r = Arc::clone(&m);
    let r = cir_trace::spawn("r", move || {
        let mut v: i32 = 0;
        {
            let _g = m_r.lock().unwrap();
        }
        cir_trace::record("channel_recv", "ch1_rx"); v = ch1_rx.recv().unwrap();
        {
            let _g = m_r.lock().unwrap();
        }
        let _ = v;
    });

    s.join().unwrap();
    r.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
