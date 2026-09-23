mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::mpsc::sync_channel;

use std::thread;

fn main() { cir_trace::init();
    let m: Mutex<i64> = Mutex::new_named("res_mutex0", 0); // guards `done`
    let (ch1_tx, ch1_rx) = sync_channel::<i64>(0); // ch1: rendezvous
    let (ch2_tx, ch2_rx) = sync_channel::<i64>(0); // ch2: rendezvous

    thread::scope(|scope| {
        let s = scope.spawn(|| {
            {
                let _guard = m.lock().unwrap();
            }
            cir_trace::record("channel_send", "ch1_tx"); ch1_tx.send(1).unwrap();
            cir_trace::record("channel_recv", "ch2_rx"); let _ack = ch2_rx.recv().unwrap();
        });

        let r = scope.spawn(|| {
            {
                let _guard = m.lock().unwrap();
            }
            cir_trace::record("channel_recv", "ch1_rx"); let _v = ch1_rx.recv().unwrap();
            cir_trace::record("channel_send", "ch2_tx"); ch2_tx.send(1).unwrap();
        });

        s.join().unwrap();
        r.join().unwrap();
    });

    let mut done = m.lock().unwrap();
    *done = 1;
    let d = *done;
    drop(done);

    println!("DONE done={}", d);
 cir_trace::finish();}
