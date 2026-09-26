mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::mpsc;
use std::sync::{Arc};

fn main() { cir_trace::init();
    let lk = Arc::new(Mutex::new_named("lk_mutex0", ()));
    let (ch1_tx, ch1_rx) = mpsc::sync_channel::<i32>(0);
    let (ch2_tx, ch2_rx) = mpsc::sync_channel::<i32>(0);

    let lk_s = Arc::clone(&lk);
    let handle_s = cir_trace::spawn("s", move || s(lk_s, ch1_tx, ch2_rx));

    let lk_r = Arc::clone(&lk);
    let handle_r = cir_trace::spawn("r", move || r(lk_r, ch1_rx, ch2_tx));

    handle_s.join().unwrap();
    handle_r.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}

fn s(lk: Arc<Mutex<()>>, ch1: mpsc::SyncSender<i32>, ch2: mpsc::Receiver<i32>) {
    let mut v = 0i32;
    {
        let _guard = lk.lock().unwrap();
    }
    cir_trace::record("channel_send", "ch1"); ch1.send(1).unwrap();
    cir_trace::record("channel_recv", "ch2"); v = ch2.recv().unwrap();
    let _ = v;
    {
        let _guard = lk.lock().unwrap();
    }
}

fn r(lk: Arc<Mutex<()>>, ch1: mpsc::Receiver<i32>, ch2: mpsc::SyncSender<i32>) {
    let mut v = 0i32;
    {
        let _guard = lk.lock().unwrap();
    }
    cir_trace::record("channel_recv", "ch1"); v = ch1.recv().unwrap();
    cir_trace::record("channel_send", "ch2"); ch2.send(1).unwrap();
    let _ = v;
    {
        let _guard = lk.lock().unwrap();
    }
}
