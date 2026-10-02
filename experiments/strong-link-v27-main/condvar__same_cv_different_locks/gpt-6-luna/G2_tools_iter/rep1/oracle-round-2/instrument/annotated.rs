mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{mpsc::{self, Receiver, SyncSender};
use std::thread;

// A shared notification resource with a wait queue for each lock.
struct Cv {
    w1: Condvar,
    w2: Condvar,
}

fn w1(m1: Arc<Mutex<bool>>, cv: Arc<Cv>, ready: SyncSender<()>) {
    let mut finished = m1.lock().unwrap();

    // Announce while holding m1, before blocking.
    cir_trace::record("channel_send", "ready"); ready.send(()).unwrap();

    while !*finished {
        finished = cv.w1.wait(finished).unwrap();
    }
}

fn w2(m2: Arc<Mutex<bool>>, cv: Arc<Cv>, ready: SyncSender<()>) {
    let mut finished = m2.lock().unwrap();

    // Announce while holding m2, before blocking.
    cir_trace::record("channel_send", "ready"); ready.send(()).unwrap();

    while !*finished {
        finished = cv.w2.wait(finished).unwrap();
    }
}

fn notifier(
    m1: Arc<Mutex<bool>>,
    m2: Arc<Mutex<bool>>,
    cv: Arc<Cv>,
    ready: Receiver<()>,
) {
    // Wait until both waiters have announced themselves.
    cir_trace::record("channel_recv", "ready"); ready.recv().unwrap();
    cir_trace::record("channel_recv", "ready"); ready.recv().unwrap();

    // Wake each waiter while holding the lock it waits on. Acquire the locks
    // separately to avoid holding both at once.
    {
        let mut finished = m1.lock().unwrap();
        *finished = true;
        cv.w1.notify_all();
    }

    {
        let mut finished = m2.lock().unwrap();
        *finished = true;
        cv.w2.notify_all();
    }
}

fn main() { cir_trace::init();
    let m1 = Arc::new(Mutex::new_named("m1_mutex0#1387", false));
    let m2 = Arc::new(Mutex::new_named("m2_mutex0#1429", false));
    let cv = Arc::new(Cv {
        w1: Condvar::new_named("w1#1490"),
        w2: Condvar::new_named("w2#1518"),
    });

    // A bounded channel acts as a counting semaphore for the announcements.
    let (ready_tx, ready_rx) = mpsc::sync_channel(2);

    let w1_handle = {
        let m1 = Arc::clone(&m1);
        let cv = Arc::clone(&cv);
        let ready = ready_tx.clone();
        cir_trace::spawn("w1#1802", move || w1(m1, cv, ready))
    };

    let w2_handle = {
        let m2 = Arc::clone(&m2);
        let cv = Arc::clone(&cv);
        let ready = ready_tx.clone();
        cir_trace::spawn("w2#1987", move || w2(m2, cv, ready))
    };

    let notifier_handle = {
        let m1 = Arc::clone(&m1);
        let m2 = Arc::clone(&m2);
        let cv = Arc::clone(&cv);
        cir_trace::spawn("notifier#2174", move || notifier(m1, m2, cv, ready_rx))
    };

    w1_handle.join().unwrap();
    w2_handle.join().unwrap();
    notifier_handle.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
