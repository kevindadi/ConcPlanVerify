// R1: The main task performs the same call sequence twice: it invokes the
//     auxiliary routine `helper`, waits for it to finish, then repeats.
// R2: The main task and `helper` share no mutexes, counters, or any other
//     shared state — `helper` touches nothing the main task owns, so no two
//     tasks ever contend for a resource.
// R3: Each auxiliary call runs to completion (the thread is joined) before
//     the main task starts the next call.
// R4: Every schedule terminates: the main task simply spawns, joins, spawns,
//     joins, prints, and exits. `helper` performs no blocking operations and
//     returns immediately, so no interleaving can deadlock or hang.
// R5: The program prints exactly one line, `DONE done=1`, and exits.

fn helper() {
    // Auxiliary routine: no shared state, no synchronization, runs to
    // completion on its own and returns.
}

fn main() {
    // First call sequence: spawn the auxiliary routine and let it run to
    // completion before proceeding.
    let first = std::thread::spawn(helper);
    first.join().expect("helper task must not panic");

    // Second call sequence: the main task begins the same call again.
    let second = std::thread::spawn(helper);
    second.join().expect("helper task must not panic");

    // R5: exactly one line of output, then the process exits.
    println!("DONE done=1");
}
