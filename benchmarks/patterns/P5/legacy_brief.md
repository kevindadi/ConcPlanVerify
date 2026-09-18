# P5 legacy brief (partial_deadlock)

Canonical requirement from the legacy corpus (draft; may reveal the
defect and therefore is NOT used as the live generation input):

Model workers A and B plus an independent bystander loop. A locks mutex A, signals semaphore A, waits on semaphore B, then locks mutex B; B does the mirror image. The bystander loops forever doing nothing. Declare goals that both workers reach their return points, and design the synchronization so those goals are reachable.
