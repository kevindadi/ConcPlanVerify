mod cir_trace;
use concir_sync::Semaphore;
use std::sync::Arc;
use std::sync::mpsc::sync_channel;
use std::thread;

fn main() { cir_trace::init();
    let m = Semaphore::new_named("m_semaphore0", 1);
    let ch = sync_channel(1);
    let (tx, rx) = ch;

    let m_sender = Arc::clone(&m);
    let sender = cir_trace::spawn("sender", move || {
        for value in [1, 2] {
            {
                let _permit = m_sender.acquire();
            }
            cir_trace::record("channel_send", "tx"); tx.send(value).unwrap();
        }
    });

    let m_receiver = Arc::clone(&m);
    let receiver = cir_trace::spawn("receiver", move || {
        for _ in 0..2 {
            cir_trace::record("channel_recv", "rx"); let value = rx.recv().unwrap();
            {
                let _permit = m_receiver.acquire();
                let _ = value;
            }
        }
    });

    sender.join().unwrap();
    receiver.join().unwrap();
    println!("DONE done=1");
 cir_trace::finish();}
