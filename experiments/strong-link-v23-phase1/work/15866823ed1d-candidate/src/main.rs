mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{mpsc, Arc};
use std::thread;

fn sender(ch: mpsc::SyncSender<u8>, m: Arc<Mutex<usize>>) {
    {
        let mut shared = m.lock().unwrap();
        *shared += 1;
    }
    cir_trace::record("channel_send", "ch"); ch.send(1).unwrap();

    {
        let mut shared = m.lock().unwrap();
        *shared += 1;
    }
    cir_trace::record("channel_send", "ch"); ch.send(2).unwrap();
}

fn receiver(ch: mpsc::Receiver<u8>, m: Arc<Mutex<usize>>) {
    {
        let mut shared = m.lock().unwrap();
        *shared += 1;
    }
    cir_trace::record("channel_recv", "ch"); let _first = ch.recv().unwrap();

    {
        let mut shared = m.lock().unwrap();
        *shared += 1;
    }
    cir_trace::record("channel_recv", "ch"); let _second = ch.recv().unwrap();
}

fn main() { cir_trace::init();
    let (tx, rx) = mpsc::sync_channel(1);
    let m = Arc::new(Mutex::new_named("m_mutex0#700", 0));

    let sender_m = Arc::clone(&m);
    let sender_thread = cir_trace::spawn("sender#769", move || sender(tx, sender_m));

    let receiver_m = Arc::clone(&m);
    let receiver_thread = cir_trace::spawn("receiver#878", move || receiver(rx, receiver_m));

    sender_thread.join().unwrap();
    receiver_thread.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
