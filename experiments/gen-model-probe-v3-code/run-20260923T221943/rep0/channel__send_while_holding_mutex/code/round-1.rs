use std::sync::mpsc::sync_channel;
use std::sync::Mutex;
use std::thread;

fn main() {
    let m: Mutex<i64> = Mutex::new(0); // guards `done`
    let (ch1_tx, ch1_rx) = sync_channel::<i64>(0); // ch1: rendezvous
    let (ch2_tx, ch2_rx) = sync_channel::<i64>(0); // ch2: rendezvous

    thread::scope(|scope| {
        let s = scope.spawn(|| {
            {
                let _guard = m.lock().unwrap();
            }
            ch1_tx.send(1).unwrap();
            let _ack = ch2_rx.recv().unwrap();
        });

        let r = scope.spawn(|| {
            {
                let _guard = m.lock().unwrap();
            }
            let _v = ch1_rx.recv().unwrap();
            ch2_tx.send(1).unwrap();
        });

        s.join().unwrap();
        r.join().unwrap();
    });

    let mut done = m.lock().unwrap();
    *done = 1;
    let d = *done;
    drop(done);

    println!("DONE done={}", d);
}
