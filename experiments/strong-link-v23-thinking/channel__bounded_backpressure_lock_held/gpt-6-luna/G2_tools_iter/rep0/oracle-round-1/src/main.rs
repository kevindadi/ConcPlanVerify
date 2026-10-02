mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{mpsc, Arc};
use std::thread;

fn sender(ch: mpsc::SyncSender<i32>, m: Arc<Mutex<usize>>) {
    for value in [1, 2] {
        {
            let mut count = m.lock().unwrap();
            *count += 1;
        } // Release m before sending, which may wait.
        cir_trace::record("channel_send", "ch"); ch.send(value).unwrap();
    }
}

fn receiver(ch: mpsc::Receiver<i32>, m: Arc<Mutex<usize>>) {
    for _ in 0..2 {
        cir_trace::record("channel_recv", "ch"); let _value = ch.recv().unwrap(); // May wait, but does not hold m.
        let mut count = m.lock().unwrap();
        *count += 1;
    }
}

fn main() { cir_trace::init();
    let (tx, rx) = mpsc::sync_channel(1);
    let m = Arc::new(Mutex::new_named("m_mutex0#630", 0));

    let sender_handle = {
        let m = Arc::clone(&m);
        cir_trace::spawn("sender#706", move || sender(tx, m))
    };
    let receiver_handle = {
        let m = Arc::clone(&m);
        cir_trace::spawn("receiver#818", move || receiver(rx, m))
    };

    sender_handle.join().unwrap();
    receiver_handle.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
