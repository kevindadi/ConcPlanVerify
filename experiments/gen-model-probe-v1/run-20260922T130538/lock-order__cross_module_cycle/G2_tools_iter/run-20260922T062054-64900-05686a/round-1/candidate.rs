use std::sync::{Arc, Mutex};
use std::thread;

mod alpha {
    use std::sync::Mutex;

    /// The shared resource owned by module alpha.
    pub struct Resource {
        pub data: Mutex<u32>,
    }

    impl Resource {
        pub fn new() -> Self {
            Resource {
                data: Mutex::new(0),
            }
        }
    }

    /// R3: this module's task declares that it depends on the
    /// resource owned by module beta.
    pub const TASK_DEPENDS_ON: &str = "beta::Resource";

    /// The task that runs in module alpha. It needs both resources.
    ///
    /// Deadlock is impossible because every task acquires the
    /// resources in the same global order: alpha first, then beta.
    pub fn task(own: &Resource, other: &crate::beta::Resource) {
        // Declare the dependency on the other module's resource.
        let _declared_dependency = TASK_DEPENDS_ON;

        // Acquire both resources in the global order (alpha -> beta).
        let mut own_guard = own.data.lock().unwrap();
        let mut other_guard = other.data.lock().unwrap();

        // Perform work while holding both resources at the same time.
        *own_guard += 1;
        *other_guard += 1;

        // Release each resource before finishing.
        drop(other_guard);
        drop(own_guard);
    }
}

mod beta {
    use std::sync::Mutex;

    /// The shared resource owned by module beta.
    pub struct Resource {
        pub data: Mutex<u32>,
    }

    impl Resource {
        pub fn new() -> Self {
            Resource {
                data: Mutex::new(0),
            }
        }
    }

    /// R3: this module's task declares that it depends on the
    /// resource owned by module alpha.
    pub const TASK_DEPENDS_ON: &str = "alpha::Resource";

    /// The task that runs in module beta. It needs both resources.
    ///
    /// It uses the same global acquisition order (alpha -> beta), so
    /// no circular wait can ever arise in any interleaving.
    pub fn task(own: &Resource, other: &crate::alpha::Resource) {
        // Declare the dependency on the other module's resource.
        let _declared_dependency = TASK_DEPENDS_ON;

        // Acquire both resources in the global order (alpha -> beta).
        let mut other_guard = other.data.lock().unwrap();
        let mut own_guard = own.data.lock().unwrap();

        // Perform work while holding both resources at the same time.
        *own_guard += 1;
        *other_guard += 1;

        // Release each resource before finishing.
        drop(own_guard);
        drop(other_guard);
    }
}

fn main() {
    // One instance of each shared resource, owned by its module.
    let alpha_resource = Arc::new(alpha::Resource::new());
    let beta_resource = Arc::new(beta::Resource::new());

    // Launch the task that runs in module alpha.
    let a1 = Arc::clone(&alpha_resource);
    let b1 = Arc::clone(&beta_resource);
    let task_alpha = thread::spawn(move || {
        alpha::task(&a1, &b1);
    });

    // Launch the task that runs in module beta.
    let a2 = Arc::clone(&alpha_resource);
    let b2 = Arc::clone(&beta_resource);
    let task_beta = thread::spawn(move || {
        beta::task(&b2, &a2);
    });

    // The starting thread only finishes after both tasks have finished.
    task_alpha.join().unwrap();
    task_beta.join().unwrap();

    println!("DONE done=1");
}
