use std::sync::{mpsc, Arc, Condvar, Mutex, MutexGuard};
use std::thread;

// Each condition variable is paired with exactly one mutex.
struct Cv {
    for_m1: Condvar,
    for_m2: Condvar,
}

fn w1(m1: Arc<Mutex<bool>>, cv: Arc<Cv>, ready: mpsc::SyncSender<()>) {
    let mut state = m1.lock().unwrap();
    ready.send(()).unwrap(); // Announce before waiting.

    while !*state {
        state = cv.for_m1.wait(state).unwrap();
    }
}

fn w2(m2: Arc<Mutex<bool>>, cv: Arc<Cv>, ready: mpsc::SyncSender<()>) {
    let mut state = m2.lock().unwrap();
    ready.send(()).unwrap(); // Announce before waiting.

    while !*state {
        state = cv.for_m2.wait(state).unwrap();
    }
}

fn notifier(
    m1: Arc<Mutex<bool>>,
    m2: Arc<Mutex<bool>>,
    cv: Arc<Cv>,
    ready_rx: mpsc::Receiver<()>,
) {
    // Wait until both waiters have announced that they are about to wait.
    ready_rx.recv().unwrap();
    ready_rx.recv().unwrap();

    // Acquiring these locks ensures each waiter has entered its condition-variable
    // wait. Set both predicates and notify while holding both locks.
    let mut state1 = m1.lock().unwrap();
    let mut state2 = m2.lock().unwrap();

    *state1 = true;
    *state2 = true;
    cv.for_m1.notify_all();
    cv.for_m2.notify_all();
}

fn main() {
    let m1 = Arc::new(Mutex::new(false));
    let m2 = Arc::new(Mutex::new(false));
    let cv = Arc::new(Cv {
        for_m1: Condvar::new(),
        for_m2: Condvar::new(),
    });

    // A capacity-two token channel serves as the ready semaphore.
    let (ready, ready_rx) = mpsc::sync_channel(2);

    let w1_handle = {
        let m1 = Arc::clone(&m1);
        let cv = Arc::clone(&cv);
        let ready = ready.clone();
        thread::spawn(move || w1(m1, cv, ready))
    };

    let w2_handle = {
        let m2 = Arc::clone(&m2);
        let cv = Arc::clone(&cv);
        let ready = ready.clone();
        thread::spawn(move || w2(m2, cv, ready))
    };

    drop(ready);

    let notifier_handle = {
        let m1 = Arc::clone(&m1);
        let m2 = Arc::clone(&m2);
        let cv = Arc::clone(&cv);
        thread::spawn(move || notifier(m1, m2, cv, ready_rx))
    };

    w1_handle.join().unwrap();
    w2_handle.join().unwrap();
    notifier_handle.join().unwrap();

    println!("DONE done=1");
}
