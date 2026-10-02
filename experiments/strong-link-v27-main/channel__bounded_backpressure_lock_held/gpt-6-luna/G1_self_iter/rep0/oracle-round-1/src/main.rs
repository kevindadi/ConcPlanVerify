mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{mpsc::{sync_channel, Receiver, SyncSender};
use std::thread;

fn touch_m(m: &Mutex<()>) {
    let guard = m.lock().expect("m poisoned");
    drop(guard);
}

fn sender(ch: SyncSender<u8>, m: Arc<Mutex<()>>) {
    touch_m(&m);
    cir_trace::record("channel_send", "ch"); ch.send(1).expect("receiver disconnected");

    touch_m(&m);
    cir_trace::record("channel_send", "ch"); ch.send(2).expect("receiver disconnected");
}

fn receiver(ch: Receiver<u8>, m: Arc<Mutex<()>>) {
    touch_m(&m);
    assert_eq!(ch.recv().expect("sender disconnected"), 1);

    touch_m(&m);
    assert_eq!(ch.recv().expect("sender disconnected"), 2);
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0#631", ()));
    let ch = sync_channel::<u8>(1);
    let (sender_ch, receiver_ch) = ch;

    let sender_m = Arc::clone(&m);
    let sender_thread = cir_trace::spawn("sender#776", move || sender(sender_ch, sender_m));

    let receiver_m = Arc::clone(&m);
    let receiver_thread = cir_trace::spawn("receiver#892", move || receiver(receiver_ch, receiver_m));

    sender_thread.join().expect("sender panicked");
    receiver_thread.join().expect("receiver panicked");

    println!("DONE done=1");
 cir_trace::finish();}
