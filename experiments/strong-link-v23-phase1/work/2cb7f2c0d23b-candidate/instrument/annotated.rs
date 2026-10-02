mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Arc};
use std::thread;

fn sender(ch: SyncSender<i32>, m: Arc<Mutex<()>>) {
    for value in [1, 2] {
        {
            let _guard = m.lock().unwrap();
        }
        cir_trace::record("channel_send", "ch"); ch.send(value).unwrap();
    }
}

fn receiver(ch: Receiver<i32>, m: Arc<Mutex<()>>) {
    for _ in 0..2 {
        cir_trace::record("channel_recv", "ch"); let _value = ch.recv().unwrap();
        let _guard = m.lock().unwrap();
    }
}

fn main() { cir_trace::init();
    let (ch, rx) = sync_channel(1);
    let m = Arc::new(Mutex::new_named("m_mutex0#528", ()));

    let sender_m = Arc::clone(&m);
    let sender_handle = cir_trace::spawn("sender#598", move || sender(ch, sender_m));

    let receiver_handle = cir_trace::spawn("receiver#670", move || receiver(rx, m));

    sender_handle.join().unwrap();
    receiver_handle.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
