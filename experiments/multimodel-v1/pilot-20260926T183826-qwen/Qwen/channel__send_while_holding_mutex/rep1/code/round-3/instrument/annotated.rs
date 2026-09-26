mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread;

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", ()));
    let (tx1, rx1): (Sender<i32>, Receiver<i32>) = channel();
    let (tx2, rx2): (Sender<i32>, Receiver<i32>) = channel();

    // s: mutex_lock main::m; mutex_unlock main::m; channel_send main::ch1; channel_recv main::ch2
    let m_s = Arc::clone(&m);
    let tx1_s = tx1;
    let rx2_s = rx2;

    let s_handle = cir_trace::spawn("s_handle", move || {
        {
            let _guard = m_s.lock().unwrap();
        }
        cir_trace::record("channel_send", "tx1_s"); tx1_s.send(1).unwrap();
        cir_trace::record("channel_recv", "rx2_s"); let _tmp_s: i32 = rx2_s.recv().unwrap();
    });

    // r: channel_recv main::ch1; mutex_lock main::m; mutex_unlock main::m; channel_send main::ch2
    let m_r = Arc::clone(&m);
    let rx1_r = rx1;
    let tx2_r = tx2;

    let r_handle = cir_trace::spawn("r_handle", move || {
        cir_trace::record("channel_recv", "rx1_r"); let _tmp_r: i32 = rx1_r.recv().unwrap();
        {
            let _guard = m_r.lock().unwrap();
        }
        cir_trace::record("channel_send", "tx2_r"); tx2_r.send(1).unwrap();
    });

    s_handle.join().unwrap();
    r_handle.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
