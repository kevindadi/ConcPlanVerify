mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{mpsc, Arc};
use std::thread;

fn sender(ch: mpsc::SyncSender<i32>, m: Arc<Mutex<()>>) {
    {
        let _guard = m.lock().unwrap();
    }
    cir_trace::record("channel_send", "ch"); ch.send(1).unwrap();

    {
        let _guard = m.lock().unwrap();
    }
    cir_trace::record("channel_send", "ch"); ch.send(2).unwrap();
}

fn receiver(ch: mpsc::Receiver<i32>, m: Arc<Mutex<()>>) {
    {
        let _guard = m.lock().unwrap();
    }
    cir_trace::record("channel_recv", "ch"); let _first = ch.recv().unwrap();

    {
        let _guard = m.lock().unwrap();
    }
    cir_trace::record("channel_recv", "ch"); let _second = ch.recv().unwrap();
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0#550", ()));
    let ch = mpsc::sync_channel(1);
    let (tx, rx) = ch;

    let sender_m = Arc::clone(&m);
    let sender_handle = cir_trace::spawn("sender#679", move || sender(tx, sender_m));

    let receiver_m = Arc::clone(&m);
    let receiver_handle = cir_trace::spawn("receiver#788", move || receiver(rx, receiver_m));

    sender_handle.join().unwrap();
    receiver_handle.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
