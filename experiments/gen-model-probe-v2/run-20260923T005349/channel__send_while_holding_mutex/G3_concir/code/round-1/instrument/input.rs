use std::sync::mpsc::sync_channel;
use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let m = Arc::new(Mutex::new(0i32));
    let ch1 = sync_channel::<i32>(0);
    let (ch1_tx, ch1_rx) = ch1;
    let ch2 = sync_channel::<i32>(0);
    let (ch2_tx, ch2_rx) = ch2;

    thread::scope(|scope| {
        let s = {
            let m = Arc::clone(&m);
            let ch1_tx = ch1_tx.clone();
            move || {
                {
                    let _guard = m.lock().unwrap();
                }
                ch1_tx.send(1).unwrap();
                let ack = ch2_rx.recv().unwrap();
                let _ = ack;
            }
        };

        let r = {
            let m = Arc::clone(&m);
            let ch2_tx = ch2_tx.clone();
            move || {
                {
                    let _guard = m.lock().unwrap();
                }
                let v = ch1_rx.recv().unwrap();
                let _ = v;
                ch2_tx.send(1).unwrap();
            }
        };

        scope.spawn(s);
        scope.spawn(r);
    });

    let done = {
        let mut done = m.lock().unwrap();
        *done = 1;
        *done
    };
    println!("DONE done={}", done);
}
