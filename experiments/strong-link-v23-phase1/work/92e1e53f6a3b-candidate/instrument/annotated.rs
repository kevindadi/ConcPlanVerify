mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{mpsc, Arc};
use std::thread;

fn sender(ch: mpsc::SyncSender<u8>, m: Arc<Mutex<usize>>) {
    cir_trace::record("channel_send", "ch"); ch.send(1).unwrap();

    let mut shared = m.lock().unwrap();
    *shared += 1;
    drop(shared);

    cir_trace::record("channel_send", "ch"); ch.send(2).unwrap();

    let mut shared = m.lock().unwrap();
    *shared += 1;
    drop(shared);
}

fn receiver(ch: mpsc::Receiver<u8>, m: Arc<Mutex<usize>>) {
    cir_trace::record("channel_recv", "ch"); let _first = ch.recv().unwrap();

    let mut shared = m.lock().unwrap();
    *shared += 1;
    drop(shared);

    cir_trace::record("channel_recv", "ch"); let _second = ch.recv().unwrap();

    let mut shared = m.lock().unwrap();
    *shared += 1;
    drop(shared);
}

fn main() { cir_trace::init();
    let (ch_sender, ch_receiver) = mpsc::sync_channel(1);
    let m = Arc::new(Mutex::new_named("m_mutex0#712", 0));

    let sender_m = Arc::clone(&m);
    let sender_thread = cir_trace::spawn("sender#781", move || sender(ch_sender, sender_m));

    let receiver_m = Arc::clone(&m);
    let receiver_thread = cir_trace::spawn("receiver#897", move || receiver(ch_receiver, receiver_m));

    sender_thread.join().unwrap();
    receiver_thread.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
