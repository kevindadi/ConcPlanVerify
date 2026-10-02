use std::sync::{Arc, Condvar, Mutex, MutexGuard};
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
    completed.fetch_add(1, Ordering::Relaxed);
    drop(permit);
}

fn run_role(s: Arc<Semaphore>, completed: Arc<AtomicUsize>) {
    let activations: Vec<_> = (0..2)
        .map(|_| {
            let s = Arc::clone(&s);
            let completed = Arc::clone(&completed);
            thread::spawn(move || activation(s, completed))
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

fn main() {
    let s = Arc::new(Semaphore::new());
    let completed = Arc::new(AtomicUsize::new(0));

    let workers = [
        {
            let s = Arc::clone(&s);
            let completed = Arc::clone(&completed);
            thread::spawn(move || w1(s, completed))
        },
        {
            let s = Arc::clone(&s);
            let completed = Arc::clone(&completed);
            thread::spawn(move || w2(s, completed))
        },
        {
            let s = Arc::clone(&s);
            let completed = Arc::clone(&completed);
            thread::spawn(move || w3(s, completed))
        },
    ];

    for worker in workers {
        worker.join().expect("worker thread panicked");
    }

    assert_eq!(completed.load(Ordering::Relaxed), 6);
    println!("DONE done=1");
}
