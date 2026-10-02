mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{mpsc::{sync_channel, Receiver, SyncSender}, Arc};
use std::thread;

fn touch_m(m: &Arc<Mutex<usize>>) {
    let mut count = m.lock().unwrap();
    *count += 1;
}

fn sender(ch: SyncSender<u8>, m: Arc<Mutex<usize>>) {
    for value in [1, 2] {
        touch_m(&m);
        cir_trace::record("channel_send", "ch"); ch.send(value).unwrap();
    }
}

fn receiver(ch: Receiver<u8>, m: Arc<Mutex<usize>>) -> (u8, u8) {
    let mut values = [0; 2];
    for value in &mut values {
        touch_m(&m);
        cir_trace::record("channel_recv", "ch"); *value = ch.recv().unwrap();
    }
    (values[0], values[1])
}

fn main() { cir_trace::init();
    let ch = sync_channel(1);
    let (ch_tx, ch_rx) = ch;
    let m = Arc::new(Mutex::new_named("m_mutex0#659", 0));

    let sender_handle = {
        let m = Arc::clone(&m);
        cir_trace::spawn("sender#735", move || sender(ch_tx, m))
    };
    let receiver_handle = {
        let m = Arc::clone(&m);
        cir_trace::spawn("receiver#850", move || receiver(ch_rx, m))
    };

    let sender_finished = sender_handle.join().is_ok();
    let receiver_got_values = receiver_handle
        .join()
        .map(|values| values == (1, 2))
        .unwrap_or(false);
    let lock_count_is_correct = *m.lock().unwrap() == 4;

    let done = usize::from(sender_finished && receiver_got_values && lock_count_is_correct);
    println!("DONE done={done}");
 cir_trace::finish();}
