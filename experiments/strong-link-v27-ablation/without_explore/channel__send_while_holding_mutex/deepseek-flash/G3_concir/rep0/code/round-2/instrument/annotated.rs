mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Arc};
use std::thread;

// Worker for the sender role (main::s).
// Shared variable n is protected by mutex m.
fn s(m: Arc<Mutex<i32>>, ch1_tx: SyncSender<i32>, ch2_rx: Receiver<i32>) {
    // mutex_lock main::m ; write_shared n = 1 ; mutex_unlock main::m
    {
        let mut n = m.lock().unwrap();
        *n = 1;
    }

    // assign_local msg = 1
    let msg: i32 = 1;

    // channel_send main::ch1 msg   (rendezvous)
    cir_trace::record("channel_send", "ch1_tx"); ch1_tx.send(msg).unwrap();

    // channel_recv main::ch2 -> ack  (rendezvous)
    cir_trace::record("channel_recv", "ch2_rx"); let ack: i32 = ch2_rx.recv().unwrap();

    // mutex_lock main::m ; write_shared n = ack ; mutex_unlock main::m
    {
        let mut n = m.lock().unwrap();
        *n = ack;
    }
}

// Worker for the receiver role (main::r).
fn r(m: Arc<Mutex<i32>>, ch1_rx: Receiver<i32>, ch2_tx: SyncSender<i32>) {
    // mutex_lock main::m ; write_shared n = 1 ; mutex_unlock main::m
    {
        let mut n = m.lock().unwrap();
        *n = 1;
    }

    // channel_recv main::ch1 -> val  (rendezvous)
    cir_trace::record("channel_recv", "ch1_rx"); let val: i32 = ch1_rx.recv().unwrap();

    // assign_local ack = val
    let ack: i32 = val;

    // channel_send main::ch2 ack   (rendezvous)
    cir_trace::record("channel_send", "ch2_tx"); ch2_tx.send(ack).unwrap();

    // mutex_lock main::m ; write_shared n = val ; mutex_unlock main::m
    {
        let mut n = m.lock().unwrap();
        *n = val;
    }
}

fn main() { cir_trace::init();
    // shared variable n, protected by mutex m
    let n = Arc::new(Mutex::new_named("n_mutex0#1495", 0));

    // ch1 and ch2 are synchronous (rendezvous) channels
    let (ch1_tx, ch1_rx): (SyncSender<i32>, Receiver<i32>) = sync_channel(0);
    let (ch2_tx, ch2_rx): (SyncSender<i32>, Receiver<i32>) = sync_channel(0);

    let n_s = Arc::clone(&n);
    let s_handle = cir_trace::spawn("s#1768", move || s(n_s, ch1_tx, ch2_rx));

    let n_r = Arc::clone(&n);
    let r_handle = cir_trace::spawn("r#1865", move || r(n_r, ch1_rx, ch2_tx));

    s_handle.join().unwrap();
    r_handle.join().unwrap();

    let done = 1;
    println!("DONE done={}", done);
 cir_trace::finish();}
