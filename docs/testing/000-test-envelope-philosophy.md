# Test Envelope Philosophy

**Document:** 000-test-envelope-philosophy.md
**Status:** Active
**Created:** 2026-01-29
**Applies to:** All shebe testing

---

## Origin: Flight Test Envelope Expansion

In aerospace engineering, the **flight envelope** defines the
boundary of conditions (speed, altitude, load factor) within
which an aircraft can safely operate. When a new aircraft is
built, its envelope exists only on paper -- theoretical limits
derived from analysis and simulation.

**Flight test** is the process of converting that theoretical
envelope into a **demonstrated envelope**: the region of
conditions that have been physically tested and confirmed.

The methodology used is called **build-up testing**:

1. Start from the center of the predicted envelope -- the
   safest, most predictable conditions
2. Expand outward in small increments toward the boundaries
3. At each increment, predict what should happen, test it,
   and validate the prediction
4. Fix problems immediately before expanding further
5. Never extrapolate -- only interpolate from tested points

The mantra is: **predict, test, validate**.

A tested aircraft is not one that has been proven to fly at
its limits. It is one where every point between the center
and the boundary has been systematically verified, building
confidence at each step.

---

## Application to Software Testing

Shebe adopts this philosophy. The "flight envelope" maps to
the space of inputs and configurations the software must
handle correctly. The "demonstrated envelope" is the region
covered by tests.

### The Envelope Model

```
                    Boundary (edge cases, limits)
                   /
        +---------/-----------------------------------+
        |        /     Theoretical Envelope           |
        |       /      (what the code should handle)  |
        |      +----------------------------------+   |
        |      |                                  |   |
        |      |    Demonstrated Envelope         |   |
        |      |    (what tests have verified)    |   |
        |      |                                  |   |
        |      |         * Center                 |   |
        |      |         (happy path)             |   |
        |      |                                  |   |
        |      +----------------------------------+   |
        |                                             |
        +---------------------------------------------+
                    Beyond Envelope (invalid inputs)
```

**Center:** Happy path -- valid inputs, typical usage,
normal configurations. This is where testing starts.

**Demonstrated envelope:** The region covered by passing
tests. Every tested condition is a known-good point.

**Boundary:** Edge cases, limits, unusual but valid inputs.
Error paths, empty collections, maximum sizes.

**Beyond envelope:** Invalid inputs, malformed data, missing
files. The system should reject these gracefully, not crash.

### The Build-Up Principle

Tests are written from the center outward:

1. **Center first** -- Happy path tests for each module.
   The most common usage patterns. These must pass before
   anything else matters.

2. **Expand toward boundaries** -- Add tests for edge cases:
   empty sessions, missing files, zero results, maximum
   chunk sizes, Unicode boundaries.

3. **Test the boundary** -- Verify behavior at exact limits:
   max file size, session name length limits, chunk at
   file end.

4. **Test beyond the boundary** -- Verify graceful rejection:
   nonexistent paths, invalid session IDs, malformed JSON,
   corrupt metadata.

Each layer depends on the one inside it. There is no value
in testing boundary conditions of a module whose happy path
is unverified.

### Predict, Test, Validate

Before writing a test, state the expected behavior:

- "Indexing 5,000 files should complete in under 4 seconds"
- "Searching a deleted session should return SessionNotFound"
- "A chunk boundary must never split a multi-byte UTF-8 char"

The test verifies the prediction. If the prediction is wrong,
either the code or the understanding is incorrect -- both are
valuable to discover.

### Fix Before Expanding

When a test fails, stop expanding. Fix the failure before
adding new tests at further boundaries. A failing test at an
inner boundary means all outer boundary tests are unreliable.

This is the software equivalent of the aerospace rule: never
expand the envelope with a known anomaly.

---

## Coverage as Envelope Area

Line coverage is a proxy for how much of the theoretical
envelope has been demonstrated. It is not a goal in itself --
it is a measurement of exploration completeness.

### Coverage Zones

| Zone | Coverage | Interpretation |
|------|----------|----------------|
| Core domain logic | >90% | Center and boundaries well-explored |
| Adapter layers (MCP, CLI) | >80% | Primary paths verified |
| Error/transport plumbing | >70% | Main error paths covered |
| Integration (server loop) | >60% | E2E paths exercised |

Low coverage in a module means unexplored envelope -- not
necessarily missing tests, but missing confidence about how
the code behaves.

### Where to Expand Next

When expanding coverage, prioritize by risk:

1. **Uncovered code that handles user data** -- chunking,
   search, indexing. This is the core envelope.
2. **Uncovered error paths** -- what happens when things fail.
   Users encounter these in production.
3. **Protocol compliance** -- MCP method routing, JSON-RPC
   correctness. Clients depend on this.
4. **Configuration and setup** -- XDG paths, config loading.
   Important but lower risk.

---

## Applying the Philosophy

### For New Features

When adding a feature (e.g. a new MCP tool):

1. Write center tests first: valid input, expected output
2. Add boundary tests: empty results, maximum input sizes
3. Add beyond-boundary tests: missing params, invalid types
4. Verify coverage of the new code reaches the zone target

### For Bug Fixes

When fixing a bug:

1. Write a test that reproduces the bug (demonstrates the
   gap in the envelope)
2. Fix the code
3. Verify the test passes
4. Consider: are there adjacent untested conditions that
   could have similar bugs? Expand the envelope there too.

### For Coverage Improvement

When the goal is to increase coverage:

1. Identify the largest uncovered regions (files with lowest
   coverage percentage)
2. Start from the center of each uncovered module -- write
   happy-path tests first
3. Expand outward to boundaries and error paths
4. Do not write tests that only exercise trivial code (getters,
   constructors) -- expand where it matters

---

## Summary

| Principle | Flight Testing | Shebe Testing |
|-----------|---------------|---------------|
| Start safe | Center of envelope | Happy path first |
| Build up | Small speed/altitude increments | Add edge cases incrementally |
| Predict first | "Aircraft should maintain control at Mach 0.85" | "Search returns empty vec for no matches" |
| Fix before expanding | Ground the aircraft on anomaly | Fix failing test before adding new ones |
| Demonstrate, don't assume | Fly it to prove it | Test it to prove it |
| Measure exploration | % of envelope demonstrated | Line coverage percentage |
| Never extrapolate | Only trust tested points | Only trust tested code paths |

The test suite is a map of explored territory. Every passing
test is a point where the code has been demonstrated to work.
The goal is not 100% coverage -- it is systematic, incremental
expansion from the center outward, with confidence at every
step.

---

## References

- EAA, "Stage Three: Expanding the Flight Envelope"
  https://www.eaa.org/eaa/aircraft-building/builderresources/next-steps-after-your-airplane-is-built/testing-articles/stage-three-expanding-the-flight-envelope
- NASA Technical Memorandum 88289, "Flight Test Techniques"
  https://ntrs.nasa.gov/api/citations/19870012475/downloads/19870012475.pdf
- Wikipedia, "Flight envelope"
  https://en.wikipedia.org/wiki/Flight_envelope
- Aerospace Testing International, "Envelope Expansion:
  Whether Big or Small, Always Build-Up"
  https://ati.mydigitalpublication.co.uk/articles/envelope-expansion-whether-big-or-small-always-build-up
