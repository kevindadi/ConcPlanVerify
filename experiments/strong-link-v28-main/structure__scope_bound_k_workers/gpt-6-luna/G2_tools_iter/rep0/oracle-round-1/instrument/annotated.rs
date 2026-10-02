mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc, MutexGuard};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

struct State {
    next_ticket: u64,
    serving: u64,
    held: bool,
}

struct Semaphore {
    state: Mutex<State>,
    changed: Condvar,
}

struct Permit {
    semaphore: Arc<Semaphore>,
}

impl Semaphore {
    fn new() -> Self {
        Self {
            state: Mutex::new(State {
                next_ticket: 0,
                serving: 0,
                held: false,
            }),
            changed: Condvar::new(),
        }
    }

    fn lock_state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn acquire(self: &Arc<Self>) -> Permit {
        let mut state = self.lock_state();
        let ticket = state.next_ticket;
        state.next_ticket += 1;

        while ticket != state.serving || state.held {
            state = self
                .changed
                .wait(state)
                .unwrap_or_else(|e| e.into_inner());
        }

        state.held = true;
        Permit {
            semaphore: Arc::clone(self),
        }
    }

    fn release(&self) {
        let mut state = self.lock_state();
        state.held = false;
        state.serving += 1;
        self.changed.notify_all();
    }
}

impl Drop for Permit {
    fn drop(&mut self) {
        self.semaphore.release();
    }
}

fn activation(s: Arc<Semaphore>, completed: Arc<AtomicUsize>) {
    let permit = s.acquire();
    { let __cpv = completed.fetch_add(1, Ordering::Relaxed); cir_trace::record_value("completed#2301", (completed.load(std::sync::atomic::Ordering::SeqCst)) as i64); __cpv };
    drop(permit);
}

fn run_role(s: Arc<Semaphore>, completed: Arc<AtomicUsize>) {
    let activations: Vec<_> = (0..2)
        .map(|_| {
            let s = Arc::clone(&s);
            let completed = Arc::clone(&completed);
            cir_trace::spawn("activation#1779", move || activation(s, completed))
        })
        .collect();

    for activation in activations {
        activation.join().expect("activation thread panicked");
    }
}

fn w1(s: Arc<Semaphore>, completed: Arc<AtomicUsize>) {
    run_role(s, completed);
}

fn w2(s: Arc<Semaphore>, completed: Arc<AtomicUsize>) {
    run_role(s, completed);
}

fn w3(s: Arc<Semaphore>, completed: Arc<AtomicUsize>) {
    run_role(s, completed);
}

fn main() { cir_trace::init();
    let s = Arc::new(Semaphore::new());
    let completed = Arc::new(AtomicUsize::new(0));

    let workers = [
        {
            let s = Arc::clone(&s);
            let completed = Arc::clone(&completed);
            cir_trace::spawn("w1#2463", move || w1(s, completed))
        },
        {
            let s = Arc::clone(&s);
            let completed = Arc::clone(&completed);
            cir_trace::spawn("w2#2624", move || w2(s, completed))
        },
        {
            let s = Arc::clone(&s);
            let completed = Arc::clone(&completed);
            cir_trace::spawn("w3#2785", move || w3(s, completed))
        },
    ];

    for worker in workers {
        worker.join().expect("worker thread panicked");
    }

    assert_eq!(completed.load(Ordering::Relaxed), 6);
    println!("DONE done=1");
 cir_trace::finish();}
