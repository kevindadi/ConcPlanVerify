mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", ()));
    let (ch1_tx, ch1_rx) = sync_channel::<i32>(0);
    let (ch2_tx, ch2_rx) = sync_channel::<i32>(0);

    let m_s = Arc::clone(&m);
    let t_s = cir_trace::spawn("s", move || s(m_s, ch1_tx, ch2_rx));

    let m_r = Arc::clone(&m);
    let t_r = cir_trace::spawn("r", move || r(m_r, ch1_rx, ch2_tx));

    let ack = t_s.join().unwrap();
    let _v = t_r.join().unwrap();
    println!("DONE done={ack}");
 cir_trace::finish();}

fn s(m: Arc<Mutex<()>>, ch1: SyncSender<i32>, ch2: Receiver<i32>) -> i32 {
    let mut ack = 0;
    let guard = m.lock().unwrap();
    drop(guard);
    cir_trace::record("channel_send", "ch1"); ch1.send(1).unwrap();
    cir_trace::record("channel_recv", "ch2"); ack = ch2.recv().unwrap();
    ack
}

fn r(m: Arc<Mutex<()>>, ch1: Receiver<i32>, ch2: SyncSender<i32>) -> i32 {
    let mut v = 0;
    let guard = m.lock().unwrap();
    drop(guard);
    cir_trace::record("channel_recv", "ch1"); v = ch1.recv().unwrap();
    cir_trace::record("channel_send", "ch2"); ch2.send(1).unwrap();
    v
}
