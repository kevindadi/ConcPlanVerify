mod cir_trace;
use concir_sync::Semaphore;
use std::sync::mpsc::sync_channel;
use std::thread;

fn main() { cir_trace::init();
    // ch: a channel that can hold at most one value.
    let (ch_tx, ch_rx) = sync_channel::<u32>(1);
    // m: one shared lock (a binary semaphore used as a mutex).
    let m = Semaphore::new_named("m_semaphore0", 1);

    let m_sender = m.clone();
    let sender = cir_trace::spawn("sender", move || {
        for v in [1u32, 2] {
            // Briefly take the shared lock, and release it BEFORE any
            // waiting on the channel, so the receiver can always get it.
            {
                let _permit = m_sender.acquire();
                // Critical section: no channel operations while holding m.
            }
            // Send; this waits while ch still holds the previous value,
            // so the second value cannot be sent before the first is taken.
            cir_trace::record("channel_send", "ch_tx"); ch_tx.send(v).expect("receiver alive");
        }
    });

    let m_receiver = m.clone();
    let receiver = cir_trace::spawn("receiver", move || {
        for _ in 0..2 {
            // Receive; this waits while ch is empty. The shared lock is
            // NOT held while waiting on the channel.
            cir_trace::record("channel_recv", "ch_rx"); let _v = ch_rx.recv().expect("sender alive");
            // Briefly take the shared lock after the channel operation.
            {
                let _permit = m_receiver.acquire();
                // Critical section: no channel operations while holding m.
            }
        }
    });

    sender.join().expect("sender panicked");
    receiver.join().expect("receiver panicked");
    println!("DONE done=1");
 cir_trace::finish();}
