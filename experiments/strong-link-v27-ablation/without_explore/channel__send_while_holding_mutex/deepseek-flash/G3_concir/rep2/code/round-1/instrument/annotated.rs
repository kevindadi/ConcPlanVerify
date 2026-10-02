mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Arc};
use std::thread;

fn s(lk: Arc<Mutex<()>>, tx1: SyncSender<i32>, rx2: Receiver<i32>) {
    {
        let _guard = lk.lock().unwrap();
    }

    cir_trace::record("channel_send", "tx1"); tx1.send(1).unwrap();

    cir_trace::record("channel_recv", "rx2"); let ack: i32 = rx2.recv().unwrap();
    let _ = ack;

    {
        let _guard = lk.lock().unwrap();
    }
}

fn r(lk: Arc<Mutex<()>>, rx1: Receiver<i32>, tx2: SyncSender<i32>) {
    {
        let _guard = lk.lock().unwrap();
    }

    cir_trace::record("channel_recv", "rx1"); let v: i32 = rx1.recv().unwrap();
    let _ = v;

    cir_trace::record("channel_send", "tx2"); tx2.send(1).unwrap();

    {
        let _guard = lk.lock().unwrap();
    }
}

fn main() { cir_trace::init();
    let lk = Arc::new(Mutex::new_named("lk_mutex0#671", ()));

    let (tx1, rx1) = sync_channel::<i32>(0);
    let (tx2, rx2) = sync_channel::<i32>(0);

    let lk_s = Arc::clone(&lk);
    let lk_r = Arc::clone(&lk);

    let hs = cir_trace::spawn("s#851", move || s(lk_s, tx1, rx2));
    let hr = cir_trace::spawn("r#906", move || r(lk_r, rx1, tx2));

    hs.join().unwrap();
    hr.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
