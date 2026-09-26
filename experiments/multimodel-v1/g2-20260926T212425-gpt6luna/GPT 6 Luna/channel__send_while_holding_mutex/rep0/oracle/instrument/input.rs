use concir_sync::Semaphore;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::sync_channel;
use std::sync::Arc;
use std::thread;

fn main() {
    let ch1 = sync_channel::<usize>(0);
    let ch2 = sync_channel::<usize>(0);

    let shared_lock = Semaphore::new(1);
    let visits = Arc::new(AtomicUsize::new(0));

    let s_lock = Arc::clone(&shared_lock);
    let s_visits = Arc::clone(&visits);
    let ch1_tx = ch1.0;
    let ch2_rx = ch2.1;
    let s = thread::spawn(move || {
        {
            let _permit = s_lock.acquire();
            s_visits.fetch_add(1, Ordering::SeqCst);
        }

        ch1_tx.send(1).unwrap();
        ch2_rx.recv().unwrap() == 2
    });

    let r_lock = Arc::clone(&shared_lock);
    let r_visits = Arc::clone(&visits);
    let ch1_rx = ch1.1;
    let ch2_tx = ch2.0;
    let r = thread::spawn(move || {
        {
            let _permit = r_lock.acquire();
            r_visits.fetch_add(1, Ordering::SeqCst);
        }

        let value = ch1_rx.recv().unwrap();
        ch2_tx.send(value + 1).unwrap();
        value == 1
    });

    let s_done = s.join().unwrap();
    let r_done = r.join().unwrap();
    let done = usize::from(
        s_done && r_done && visits.load(Ordering::SeqCst) == 2,
    );

    println!("DONE done={done}");
}
