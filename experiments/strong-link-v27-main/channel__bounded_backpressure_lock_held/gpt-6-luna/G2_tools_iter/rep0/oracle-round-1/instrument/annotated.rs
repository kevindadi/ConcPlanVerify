mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{mpsc, Arc};
use std::thread;

fn sender(ch: mpsc::SyncSender<usize>, m: Arc<Mutex<usize>>) {
    for value in [1, 2] {
        {
            let mut count = m.lock().unwrap();
            *count += 1;
        } // Release m before sending, which may block.
        cir_trace::record("channel_send", "ch"); ch.send(value).unwrap();
    }
}

fn receiver(ch: mpsc::Receiver<usize>, m: Arc<Mutex<usize>>) {
    for _ in 0..2 {
        cir_trace::record("channel_recv", "ch"); let _value = ch.recv().unwrap(); // Take the value before acquiring m.
        {
            let mut count = m.lock().unwrap();
            *count += 1;
        }
    }
}

fn main() { cir_trace::init();
    let (ch, receiver_ch) = mpsc::sync_channel(1);
    let m = Arc::new(Mutex::new_named("m_mutex0#676", 0usize));

    let sender_m = Arc::clone(&m);
    let sender_thread = cir_trace::spawn("sender#750", move || sender(ch, sender_m));

    let receiver_m = Arc::clone(&m);
    let receiver_thread = cir_trace::spawn("receiver#859", move || receiver(receiver_ch, receiver_m));

    sender_thread.join().unwrap();
    receiver_thread.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
