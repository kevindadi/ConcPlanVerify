lock
while *permits == 0 { wait }  // wait atomically unlocks and re-locks
*permits -= 1
unlock
