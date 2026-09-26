mod cir_trace;
use concir_sync::Semaphore;
use std::sync::mpsc::sync_channel;
use std::thread;

fn main() { cir_trace::init();
    // The one shared lock used by both roles (a binary semaphore).
    let lock = Semaphore::new_named("lock_semaphore0", 1);

    // ch1: rendezvous (zero-capacity) channel, s -> r.
    let (tx1, rx1) = sync_channel::<u32>(0);
    // ch2: rendezvous (zero-capacity) channel, r -> s.
    let (tx2, rx2) = sync_channel::<u32>(0);

    let lock_s = lock.clone();
    let lock_r = lock.clone();

    // Sender role.
    let s = cir_trace::spawn("s", move || {
        // Use the shared lock, but release it before any channel wait.
        {
            let _permit = lock_s.acquire();
            // Critical section: nothing blocking happens here.
        }

        // Rendezvous with r on ch1: send the value (blocks until r receives).
        cir_trace::record("channel_send", "tx1"); tx1.send(7).expect("s: send on ch1");
        // Rendezvous with r on ch2: receive the reply (blocks until r sends).
        cir_trace::record("channel_recv", "rx2"); let reply = rx2.recv().expect("s: recv on ch2");
        let _ = reply;

        // Use the shared lock again, after the channel operations.
        {
            let _permit = lock_s.acquire();
            // Critical section.
        }
    });

    // Receiver role.
    let r = cir_trace::spawn("r", move || {
        // Use the shared lock, but release it before any channel wait.
        {
            let _permit = lock_r.acquire();
            // Critical section.
        }

        // Rendezvous with s on ch1: receive the value.
        cir_trace::record("channel_recv", "rx1"); let v = rx1.recv().expect("r: recv on ch1");
        // Rendezvous with s on ch2: send the reply.
        cir_trace::record("channel_send", "tx2"); tx2.send(v + 1).expect("r: send on ch2");

        // Use the shared lock again, after the channel operations.
        {
            let _permit = lock_r.acquire();
            // Critical section.
        }
    });

    s.join().expect("join s");
    r.join().expect("join r");

    println!("DONE done=1");
 cir_trace::finish();}
