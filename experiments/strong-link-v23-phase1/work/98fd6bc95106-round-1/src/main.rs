mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::mpsc::{Receiver, SyncSender};
use std::sync::{sync_channel, Arc};

fn sender(m: Arc<Mutex<()>>, ch: SyncSender<i32>) {
    drop(m.lock().unwrap());
    cir_trace::record("channel_send", "ch"); ch.send(1).unwrap();

    drop(m.lock().unwrap());
    cir_trace::record("channel_send", "ch"); ch.send(2).unwrap();
}

fn receiver(m: Arc<Mutex<()>>, ch: Receiver<i32>) {
    drop(m.lock().unwrap());
    cir_trace::record("channel_recv", "ch"); let _first = ch.recv().unwrap();

    drop(m.lock().unwrap());
    cir_trace::record("channel_recv", "ch"); let _second = ch.recv().unwrap();
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0#482", ()));
    let (tx, rx) = sync_channel(1);

    let sender_m = Arc::clone(&m);
    let sender_thread = cir_trace::spawn("sender#588", move || sender(sender_m, tx));

    let receiver_m = Arc::clone(&m);
    let receiver_thread = cir_trace::spawn("receiver#702", move || receiver(receiver_m, rx));

    sender_thread.join().unwrap();
    receiver_thread.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
