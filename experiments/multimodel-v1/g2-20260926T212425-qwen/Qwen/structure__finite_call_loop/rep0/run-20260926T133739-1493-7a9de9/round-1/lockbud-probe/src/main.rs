use concir_sync::Semaphore;

fn helper(sem: &Semaphore) {
    let _permit = sem.acquire();
}

fn main() {
    let sem = Semaphore::new(1);
    
    // First call sequence
    helper(&sem);
    
    // Second call sequence (same as first, runs to completion before next starts per R3)
    helper(&sem);
    
    println!("DONE done=1");
}
