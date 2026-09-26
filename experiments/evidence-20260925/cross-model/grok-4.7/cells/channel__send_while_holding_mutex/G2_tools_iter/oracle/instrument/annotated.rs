mod cir_trace;
use concir_sync::Semaphore;
use std::sync::mpsc::sync_channel;
use std::sync::Arc;
use std::thread;

fn main() { cir_trace::init();
    let ch1 = sync_channel::<i32>(0);
    let ch2 = sync_channel::<i32>(0);
    let lock = Semaphore::new_named("lock_semaphore0", 1);

    let (ch1_tx, ch1_rx) = ch1;
    let (ch2_tx, ch2_rx) = ch2;
    let lock_s = Arc::clone(&lock);
    let lock_r = Arc::clone(&lock);

    let s = cir_trace::spawn("s", move || {
        let permit = lock_s.acquire();
        permit.release();
        cir_trace::record("channel_send", "ch1_tx"); ch1_tx.send(1).unwrap();
        cir_trace::record("channel_recv", "ch2_rx"); ch2_rx.recv().unwrap();
    });

    let r = cir_trace::spawn("r", move || {
        let permit = lock_r.acquire();
        permit.release();
        cir_trace::record("channel_recv", "ch1_rx"); let done = ch1_rx.recv().unwrap();
        cir_trace::record("channel_send", "ch2_tx"); ch2_tx.send(done).unwrap();
        done
    });

    s.join().unwrap();
    let done = r.join().unwrap();
    println!("DONE done={done}");
 cir_trace::finish();}
