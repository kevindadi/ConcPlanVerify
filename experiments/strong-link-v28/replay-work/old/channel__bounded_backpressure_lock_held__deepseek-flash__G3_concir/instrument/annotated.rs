mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::sync::mpsc;
use std::thread;

fn sender(m: Arc<Mutex<()>>, ch: mpsc::SyncSender<i32>) {
    let guard = m.lock().unwrap();
    drop(guard);

    cir_trace::record("channel_send", "ch"); ch.send(1).unwrap();
    cir_trace::record("channel_send", "ch"); ch.send(2).unwrap();
}

fn receiver(m: Arc<Mutex<()>>, ch: mpsc::Receiver<i32>) {
    let guard = m.lock().unwrap();
    drop(guard);

    let mut v1 = 0;
    let mut v2 = 0;
    cir_trace::record("channel_recv", "ch"); v1 = ch.recv().unwrap();
    cir_trace::record("channel_recv", "ch"); v2 = ch.recv().unwrap();
    let _ = (v1, v2);
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0#506", ()));
    let (ch_tx, ch_rx) = mpsc::sync_channel::<i32>(1);

    let m_sender = Arc::clone(&m);
    let sender_handle = cir_trace::spawn("sender#631", move || sender(m_sender, ch_tx));

    let m_receiver = Arc::clone(&m);
    let receiver_handle = cir_trace::spawn("receiver#743", move || receiver(m_receiver, ch_rx));

    sender_handle.join().unwrap();
    receiver_handle.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
