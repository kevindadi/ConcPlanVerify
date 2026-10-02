mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Arc};
use std::thread;

fn sender(ch: SyncSender<u8>, m: Arc<Mutex<()>>) {
    {
        let _guard = m.lock().unwrap();
    }
    cir_trace::record("channel_send", "ch"); ch.send(1).unwrap();

    {
        let _guard = m.lock().unwrap();
    }
    cir_trace::record("channel_send", "ch"); ch.send(2).unwrap();
}

fn receiver(ch: Receiver<u8>, m: Arc<Mutex<()>>) {
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
    let (ch, receiver_ch) = sync_channel(1);
    let m = Arc::new(Mutex::new_named("m_mutex0#634", ()));

    let sender_m = Arc::clone(&m);
    let sender_thread = cir_trace::spawn("sender#704", move || sender(ch, sender_m));

    let receiver_thread = cir_trace::spawn("receiver#776", move || receiver(receiver_ch, m));

    sender_thread.join().unwrap();
    receiver_thread.join().unwrap();

    let done = 1;
    println!("DONE done={done}");
 cir_trace::finish();}
