mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{mpsc, Arc};
use std::thread;

fn sender(m: Arc<Mutex<()>>, ch: mpsc::SyncSender<i32>) {
    let guard = m.lock().unwrap();
    drop(guard);
    cir_trace::record("channel_send", "ch"); ch.send(1).unwrap();

    let guard = m.lock().unwrap();
    drop(guard);
    cir_trace::record("channel_send", "ch"); ch.send(2).unwrap();
}

fn receiver(m: Arc<Mutex<()>>, ch: mpsc::Receiver<i32>) {
    cir_trace::record("channel_recv", "ch"); let first = ch.recv().unwrap();

    let guard = m.lock().unwrap();
    drop(guard);

    cir_trace::record("channel_recv", "ch"); let second = ch.recv().unwrap();
    let _ = (first, second);
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0#526", ()));
    let (tx, rx) = mpsc::sync_channel(1);

    let sender_m = Arc::clone(&m);
    let sender_handle = cir_trace::spawn("sender#638", move || sender(sender_m, tx));

    let receiver_handle = cir_trace::spawn("receiver#710", move || receiver(m, rx));

    sender_handle.join().unwrap();
    receiver_handle.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
