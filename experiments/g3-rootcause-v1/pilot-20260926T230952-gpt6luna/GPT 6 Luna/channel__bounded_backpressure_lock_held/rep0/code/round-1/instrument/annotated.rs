mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Arc};
use std::thread;

fn sender(m: Arc<Mutex<()>>, ch: SyncSender<i32>) {
    let guard = m.lock().unwrap();
    drop(guard);

    cir_trace::record("channel_send", "ch"); ch.send(1).unwrap();
    cir_trace::record("channel_send", "ch"); ch.send(2).unwrap();
}

fn receiver(m: Arc<Mutex<()>>, ch: Receiver<i32>) {
    let guard = m.lock().unwrap();
    drop(guard);

    cir_trace::record("channel_recv", "ch"); let first = ch.recv().unwrap();
    cir_trace::record("channel_recv", "ch"); let second = ch.recv().unwrap();
    let _ = (first, second);
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", ()));
    let (ch_sender, ch_receiver) = sync_channel(1);

    let sender_thread = {
        let m = Arc::clone(&m);
        cir_trace::spawn("sender", move || sender(m, ch_sender))
    };
    let receiver_thread = {
        let m = Arc::clone(&m);
        cir_trace::spawn("receiver", move || receiver(m, ch_receiver))
    };

    sender_thread.join().unwrap();
    receiver_thread.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
