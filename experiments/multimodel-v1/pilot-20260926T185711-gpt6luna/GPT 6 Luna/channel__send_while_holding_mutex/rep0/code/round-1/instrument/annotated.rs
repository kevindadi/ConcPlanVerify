mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Arc};
use std::thread;

fn s(m: Arc<Mutex<i32>>, ch1: SyncSender<i32>, ch2: Receiver<i32>) {
    {
        let mut n = m.lock().unwrap();
        *n = 1;
    }
    cir_trace::record("channel_send", "ch1"); ch1.send(1).unwrap();
    cir_trace::record("channel_recv", "ch2"); let reply = ch2.recv().unwrap();
    let _ = reply;
}

fn r(m: Arc<Mutex<i32>>, ch1: Receiver<i32>, ch2: SyncSender<i32>) {
    cir_trace::record("channel_recv", "ch1"); let received = ch1.recv().unwrap();
    {
        let mut n = m.lock().unwrap();
        *n = 2;
    }
    cir_trace::record("channel_send", "ch2"); ch2.send(received).unwrap();
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", 0));
    let (ch1_tx, ch1_rx) = sync_channel::<i32>(0);
    let (ch2_tx, ch2_rx) = sync_channel::<i32>(0);

    let s_handle = {
        let m = Arc::clone(&m);
        cir_trace::spawn("s", move || s(m, ch1_tx, ch2_rx))
    };
    let r_handle = cir_trace::spawn("r", move || r(m, ch1_rx, ch2_tx));

    s_handle.join().unwrap();
    r_handle.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
