use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

/// R3: a task declares that it depends on a resource owned by the other module.
trait DependsOn {
    type Resource;
}

mod module_a {
    use super::DependsOn;
    use std::sync::Mutex;

    /// R1: the first shared resource, owned by module A.
    pub struct ResourceA {
        pub value: Mutex<u64>,
    }

    impl ResourceA {
        pub fn new() -> Self {
            ResourceA { value: Mutex::new(0) }
        }
    }

    /// R2: the task that runs in module A.
    pub struct TaskA;

    /// R3: TaskA declares that it depends on the resource owned by module B.
    impl DependsOn for TaskA {
        type Resource = crate::module_b::ResourceB;
    }

    impl TaskA {
        /// R4: TaskA needs BOTH resources at the same time to do its work.
        ///
        /// R5/R9: deadlock is impossible because every task acquires the
        /// resources in the same global order: ResourceA first, ResourceB
        /// second. A circular wait can therefore never form.
        pub fn run(
            a: &ResourceA,
            b: &<Self as DependsOn>::Resource,
            done: &std::sync::atomic::AtomicU64,
        ) {
            // Acquire both resources in the canonical global order.
            let mut a_guard = a.value.lock().unwrap();
            let mut b_guard = b.value.lock().unwrap();

            // Critical section: both resources are held simultaneously.
            *a_guard += 1;
            *b_guard += 10;

            // R6: release each resource before finishing.
            drop(b_guard);
            drop(a_guard);

            done.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
    }
}

mod module_b {
    use super::DependsOn;
    use std::sync::Mutex;

    /// R1: the second shared resource, owned by module B.
    pub struct ResourceB {
        pub value: Mutex<u64>,
    }

    impl ResourceB {
        pub fn new() -> Self {
            ResourceB { value: Mutex::new(0) }
        }
    }

    /// R2: the task that runs in module B.
    pub struct TaskB;

    /// R3: TaskB declares that it depends on the resource owned by module A.
    impl DependsOn for TaskB {
        type Resource = crate::module_a::ResourceA;
    }

    impl TaskB {
        /// R4: TaskB also needs BOTH resources at the same time.
        ///
        /// It uses the SAME global acquisition order as TaskA
        /// (ResourceA before ResourceB), so no deadlock cycle can arise (R5).
        pub fn run(
            a: &<Self as DependsOn>::Resource,
            b: &ResourceB,
            done: &std::sync::atomic::AtomicU64,
        ) {
            // Same canonical global order: ResourceA first, then ResourceB.
            let mut a_guard = a.value.lock().unwrap();
            let mut b_guard = b.value.lock().unwrap();

            // Critical section: both resources are held simultaneously.
            *a_guard += 100;
            *b_guard += 1000;

            // R6: release each resource before finishing.
            drop(b_guard);
            drop(a_guard);

            done.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
    }
}

fn main() {
    // R1: one instance of each shared resource, owned by its module.
    let resource_a = Arc::new(module_a::
