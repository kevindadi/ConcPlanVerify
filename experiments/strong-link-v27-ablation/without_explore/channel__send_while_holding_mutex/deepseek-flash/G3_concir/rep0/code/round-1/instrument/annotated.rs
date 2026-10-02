mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0#146", 0));

    let (ch1_tx, ch1_rx): (SyncSender<i32>, Receiver<i32>) = sync_channel(0);
    let (ch2_tx, ch2_rx): (SyncSender<i32>, Receiver<i32>) = sync_channel(0);

    let m_s = Arc::clone(&m);
    let s_handle = cir_trace::spawn("s_handle#362", move || {
        {
            let mut n = m_s.lock().unwrap();
            *n = 1;
        }

        let msg = 1;
        cir_trace::record("channel_send", "ch1_tx"); ch1_tx.send(msg).unwrap();

        cir_trace::record("channel_recv", "ch2_rx"); let ack = ch2_rx.recv().unwrap();

        {
            let mut n = m_s.lock().unwrap();
            *n = ack;
        }
    });

    let m_r = Arc::clone(&m);
    let r_handle = cir_trace::spawn("r_handle#717", move || {
        {
            let mut n = m_r.lock().unwrap();
            *n = 1;
        }

        cir_trace::record("channel_recv", "ch1_rx"); let val = ch1_rx.recv().unwrap();
        let ack = val;
        cir_trace::record("channel_send", "ch2_tx"); ch2_tx.send(ack).unwrap();

        {
            let mut n = m_r.lock().unwrap();
            *n = val;
        }
    });

    s_handle.join().unwrap();
    r_handle.join().unwrap();

    let done = 1;
    println!("DONE done={}", done);
 cir_trace::finish();}
