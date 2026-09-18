# P8 legacy brief (cas_race)

Canonical requirement from the legacy corpus (draft; may reveal the
defect and therefore is NOT used as the live generation input):

Model two threads that both try to flip a shared atomic boolean from false to true using compare-and-swap, then branch on whether they won the race. Exactly one CAS succeeds; both threads must terminate on either branch. The program is race-free by construction and must verify safe.
