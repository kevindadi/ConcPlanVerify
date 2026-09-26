mod cir_trace;
use concir_sync::Semaphore;
use std::sync::{Arc, mpsc::sync_channel};
use std::thread;

fn main() { cir_trace::init();
    let slots = Semaphore::new_named("slots_semaphore0", 2);
    let slots_s1 = Arc::clone(&slots);
    let slots_r = slots;

    let ch = sync_channel::<i32>(0);
    let (tx, rx) = ch;

    let s1 = cir_trace::spawn("s1", move || {
        let _permit = slots_s1.acquire();
        cir_trace::record("channel_send", "tx"); tx.send(1).unwrap();
    });

    let r = cir_trace::spawn("r", move || {
        let _permit = slots_r.acquire();
        cir_trace::record("channel_recv", "rx"); rx.recv().unwrap()
    });

    s1.join().unwrap();
    let done = r.join().unwrap();
    println!("DONE done={}", done);
 cir_trace::finish();}
