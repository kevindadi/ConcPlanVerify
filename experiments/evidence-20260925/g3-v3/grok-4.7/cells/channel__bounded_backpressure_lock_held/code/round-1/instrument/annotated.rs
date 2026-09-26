mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{mpsc};
use std::thread;

fn main() { cir_trace::init();
    let m = Mutex::new_named("m_mutex0", ());
    let (tx, rx) = mpsc::sync_channel::<i32>(1);

    thread::scope(|scope| {
        scope.spawn(|| sender(&m, &tx));
        scope.spawn(|| receiver(&m, &rx));
    });

    println!("DONE done=1");
 cir_trace::finish();}

fn sender(m: &Mutex<()>, ch: &mpsc::SyncSender<i32>) {
    {
        let _guard = m.lock().unwrap();
    }
    cir_trace::record("channel_send", "ch"); ch.send(1).unwrap();
    {
        let _guard = m.lock().unwrap();
    }
    cir_trace::record("channel_send", "ch"); ch.send(2).unwrap();
}

fn receiver(m: &Mutex<()>, ch: &mpsc::Receiver<i32>) {
    let mut v: i32 = 0;
    {
        let _guard = m.lock().unwrap();
    }
    cir_trace::record("channel_recv", "ch"); v = ch.recv().unwrap();
    {
        let _guard = m.lock().unwrap();
    }
    cir_trace::record("channel_recv", "ch"); v = ch.recv().unwrap();
    let _ = v;
}
