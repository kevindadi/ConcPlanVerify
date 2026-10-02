mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Arc};
use std::thread;

fn sender(ch: SyncSender<usize>, m: Arc<Mutex<usize>>) {
    for value in [1, 2] {
        {
            let mut count = m.lock().unwrap();
            *count += 1;
        } // Release m before sending, which may wait.
        cir_trace::record("channel_send", "ch"); ch.send(value).unwrap();
    }
}

fn receiver(ch: Receiver<usize>, m: Arc<Mutex<usize>>) {
    for _ in 0..2 {
        {
            let mut count = m.lock().unwrap();
            *count += 1;
        } // Release m before receiving, which may wait.
        cir_trace::record("channel_recv", "ch"); let _value = ch.recv().unwrap();
    }
}

fn main() { cir_trace::init();
    let ch = sync_channel(1);
    let (tx, rx) = ch;
    let m = Arc::new(Mutex::new_named("m_mutex0#727", 0));

    let sender_thread = {
        let m = Arc::clone(&m);
        cir_trace::spawn("sender#803", move || sender(tx, m))
    };
    let receiver_thread = cir_trace::spawn("receiver#873", move || receiver(rx, m));

    sender_thread.join().unwrap();
    receiver_thread.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
