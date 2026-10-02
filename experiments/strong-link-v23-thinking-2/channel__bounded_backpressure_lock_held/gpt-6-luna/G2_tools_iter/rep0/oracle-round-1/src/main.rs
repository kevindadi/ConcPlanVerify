mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Arc};
use std::thread;

fn sender(ch: SyncSender<u32>, m: Arc<Mutex<()>>) {
    for value in [1, 2] {
        {
            let _guard = m.lock().unwrap();
        }
        cir_trace::record("channel_send", "ch"); ch.send(value).unwrap();
    }
}

fn receiver(ch: Receiver<u32>, m: Arc<Mutex<()>>) {
    for _ in 0..2 {
        cir_trace::record("channel_recv", "ch"); let _value = ch.recv().unwrap();
        {
            let _guard = m.lock().unwrap();
        }
    }
}

fn main() { cir_trace::init();
    let ch = sync_channel::<u32>(1);
    let (tx, rx) = ch;
    let m = Arc::new(Mutex::new_named("m_mutex0#576", ()));

    let sender_m = Arc::clone(&m);
    let sender_thread = cir_trace::spawn("sender#646", move || sender(tx, sender_m));

    let receiver_m = Arc::clone(&m);
    let receiver_thread = cir_trace::spawn("receiver#755", move || receiver(rx, receiver_m));

    sender_thread.join().unwrap();
    receiver_thread.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
