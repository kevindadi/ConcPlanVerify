use std::sync::{mpsc, Arc, Mutex};
use std::thread;

// The mutex guard is confined entirely to this helper: it is acquired and
// released inside the function, so no blocking channel operation (send/recv)
// can ever occur while the lock is held, in any thread.
fn bump(shared: &Mutex<i32>, delta: i32) {
    let mut v = shared.lock().unwrap();
    *v += delta;
    drop(v);
}

fn main() {
    // Rendezvous channel: send blocks until the receiver picks it up.
    let (tx, rx) = mpsc::sync_channel::<i32>(0);

    // Shared mutex used by both threads.
    let shared = Arc::new(Mutex::new(0i32));

    let s_shared = Arc::clone(&shared);
    let sender = thread::spawn(move || {
        for i in 1..=10 {
            // Lock is taken and released inside bump(); by the time this
            // line returns, the mutex is definitely unlocked.
            bump(&s_shared, i);
            // Never blocks on send while holding the lock.
            tx.send(i).unwrap();
        }
        // tx dropped here: receiver's recv loop will end.
    });

    let r_shared = Arc::clone(&shared);
    let receiver = thread::spawn(move || {
        let mut total = 0;
        // The receiver blocks on the channel WITHOUT holding the mutex.
        // It only acquires the lock (inside bump) after a value arrives,
        // and the guard is released before the next blocking recv.
        loop {
            match rx.recv() {
                Ok(i) => {
                    total += i;
                    bump(&r_shared, 1);
                }
                Err(_) => break,
            }
        }
        total
    });

    sender.join().unwrap();
    let total = receiver.join().unwrap();

    {
        let v = shared.lock().unwrap();
        // Sender added 1..=10 (55); receiver incremented once per message (10).
        assert_eq!(*v, 65);
        assert_eq!(total, 55);
        drop(v);
    }

    println!("DONE done=1");
}
