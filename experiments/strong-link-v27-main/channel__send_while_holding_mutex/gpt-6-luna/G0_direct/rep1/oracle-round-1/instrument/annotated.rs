mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{mpsc::{sync_channel, Receiver, SyncSender};
use std::thread;

fn s(ch1: SyncSender<u32>, ch2: Receiver<u32>, lock: Arc<Mutex<()>>) {
    {
        let _guard = lock.lock().unwrap();
    }

    cir_trace::record("channel_send", "ch1"); ch1.send(42).unwrap();
    cir_trace::record("channel_recv", "ch2"); ch2.recv().unwrap();
}

fn r(ch1: Receiver<u32>, ch2: SyncSender<u32>, lock: Arc<Mutex<()>>) {
    {
        let _guard = lock.lock().unwrap();
    }

    cir_trace::record("channel_recv", "ch1"); let value = ch1.recv().unwrap();

    {
        let _guard = lock.lock().unwrap();
    }

    cir_trace::record("channel_send", "ch2"); ch2.send(value).unwrap();
}

fn main() { cir_trace::init();
    let (ch1_tx, ch1_rx) = sync_channel(0);
    let (ch2_tx, ch2_rx) = sync_channel(0);
    let lock = Arc::new(Mutex::new_named("lock_mutex0#669", ()));

    let s_handle = cir_trace::spawn("s#699", move || s(ch1_tx, ch2_rx, Arc::clone(&lock)));
    let r_handle = cir_trace::spawn("r#779", move || r(ch1_rx, ch2_tx, lock));

    s_handle.join().unwrap();
    r_handle.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
