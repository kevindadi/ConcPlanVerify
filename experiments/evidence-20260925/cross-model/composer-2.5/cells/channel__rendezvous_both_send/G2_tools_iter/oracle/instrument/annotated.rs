mod cir_trace;
use concir_sync::Semaphore;
use std::sync::mpsc;
use std::thread;

fn main() { cir_trace::init();
    let ch = mpsc::sync_channel::<i32>(0);
    let start = Semaphore::new_named("start_semaphore0", 2);

    let tx = ch.0;
    let start_s1 = start.clone();
    let s1 = cir_trace::spawn("s1", move || {
        let _permit = start_s1.acquire();
        cir_trace::record("channel_send", "tx"); let _ = tx.send(1);
    });

    let rx = ch.1;
    let start_r = start.clone();
    let r = cir_trace::spawn("r", move || {
        let _permit = start_r.acquire();
        cir_trace::record("channel_recv", "rx"); let _ = rx.recv().unwrap();
    });

    s1.join().unwrap();
    r.join().unwrap();
    println!("DONE done=1");
 cir_trace::finish();}
