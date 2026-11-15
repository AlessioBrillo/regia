# DAT-01 results, increment 2 (part 1) — 2026-11-15

Covers what increment 2 has run so far: HLC merge and skew (#31), a real process kill for S-07 and concurrent
processes for S-12 (#31), and now S-09 (member revoked while a device is offline) and S-10 (object moved to another
circle). Not run yet: S-06, S-08 and alternative C. Increment 1 is in [`results-2026-11-13.md`](results-2026-11-13.md).
Rules are the owner's, frozen before results ([card](../DAT-01-data-model-sync.md), #30 and #33).

```text
cargo test -p regia-spike-dat01        # includes tests/membership.rs (S-09, S-10), tests/processes.rs (S-07, S-12)
```

## Setup

| Item | Value |
|---|---|
| Alternatives | A: `automerge` 0.12.0, one document per circle. B: `rusqlite` 0.42.0, immutable op log with version vectors |
| Toolchain | `rust:1.98.1` container (the CI image) |
| Dataset | Hand-built objects in two circles (devices 0..3), synthetic strings, canary titles. Not the generated dataset |
| Time | Simulated clock, in-process, no network |
| Data | Synthetic only |

## Gate (pass/fail)

| Check | A | B |
|---|---|---|
| S-09 offline edits of a revoked device are held aside, not visible, not lost (the text is still in the store) | pass | pass |
| S-09 what the revoker had seen before the revocation stays visible | pass | pass |
| S-09 an op that reached a peer before the revocation reached it is held aside once it does | pass | pass |
| S-09 a revoked device writing with a clock in the past cannot get in | pass | pass |
| S-09 the revoked device receives nothing created after the revocation (canary) | pass | pass |
| S-10 old circle: no view, no index entry; the new circle shows it; later changes never reach old-only members | pass | pass |
| S-10 edit made in the old circle without seeing the move: quarantined, author told, not in the new circle | pass | pass |
| S-10 move with open conflicts refused, then allowed once the field is written again | pass | pass |
| S-10 the mover must hold both circles | pass | pass |
| Increment 1 scenarios, differential property test, #31 clock/skew, S-07 kill, S-12, on the final tree | pass | pass |

"Author told": the quarantine entry carries the author's device and every member sees it after sync, the author
included. There is no separate notification channel in the spike.

## How the rules are built

- **The cut is the author's own sequence number, not a clock.** The revocation stores how many of the member's own
  writes the revoker had seen (B: ops, from the version vector; A: Automerge changes). Anything beyond it is held
  aside, wherever it arrives from. A clock set in the past changes nothing because the test never reads a clock.
- **Nothing is deleted.** Both keep the held-aside writes in the store, so the canary scan and "not lost" checks see
  them, and an admin could review them. The visible state is derived without them.
- **Moves.** The old circle gets a `__moved` register next to the object; the new circle gets a plain create with the
  visible fields. Edits the move had not seen are found the same way S-05 finds edits a delete had not seen.

## Sensitivity (the checks can fail)

Each mutation was applied to the committed source, the S-09/S-10 tests run, and the source restored.

| Mutation | Result |
|---|---|
| B: revocation cut ignored (every op shown) | Caught: 5 tests fail (4 B scenarios and the B dependent-edit test) |
| A: replay without the revoked changes disabled | Caught: 5 tests fail (4 A scenarios and the A dependent-edit test) |
| Moved object still indexed in the old circle | Caught: `s10_move_leaves_no_residue` fails on A and B |
| B: unseen edit of a moved object reported with the wrong reason | Caught: 1 test fails |
| B: relay syncs the union of scopes **and** the replica accepts circles it does not hold (a real leak) | Caught: 5 tests fail, including `s10_move_leaves_no_residue` on both A and B. That test holds the canary check; I did not isolate which assertion fired |

The last row closes the limit stated in increment 1 (B's own refusal hid a broken relay): with that refusal removed,
the leak is now seen.

## Findings that matter for the ADRs

1. **Automerge cannot take a change out of a history that depends on it.** To honour "refuse what the revoker had not
   seen", A replays the document without the cut changes and without every change built on them. An honest edit made
   by a member on top of a revoked device's unseen edit is held aside too (reason `Dependent`). B filters per op, so
   that edit stays visible. Both behaviours are pinned by tests (`s09_dependent_edit_*`). Whether holding it aside is
   acceptable is a product question for ADR-003; B does not force it.
2. **A pays a replay per read once anyone is revoked.** Every derive after a revocation rebuilds the circle document.
   Not measured here (no benchmark was run for this increment); on a large circle it is the likely cost to measure.
3. **Quarantined writes still travel.** Both alternatives forward held-aside writes to the other members during sync,
   because the raw store is what syncs. B could withhold them (its sync is its own code); A cannot, as the Automerge
   sync protocol sends everything it holds. Not implemented in B, so the two behave alike.
4. **A move is one transaction in B and two commits in A.** A crash between the two commits would leave the object in
   both circles. Not tested.
5. **The differential property test does not cover revocation or moves.** A and B legitimately differ on dependent
   edits (finding 1), so equal-state is not the right property there. These scenarios are checked with explicit
   expectations on both instead.

## Failed cases, limits, anomalies

- One admin, no policy on who may revoke or move (POL-01 and ADR-005 are out of scope here). Any member can.
- A revoked device's upload is accepted from any author. A real system needs signed authorship so a revoked device
  cannot forge ops as someone else (CRY-01, ADR-004). Not covered.
- No re-admission. Two concurrent revocations of one member keep the stricter cut; not tested.
- A moved object cannot be moved back, and two concurrent moves of one object to different circles are not handled.
- The upload goes through the same skew check as a sync message in both alternatives. A skewed upload is untested.
- Data is plaintext; encryption is CRY-01.
- The earlier limits of increment 1 that are not named above still stand.

## Outcome

Neither alternative is rejected and neither is recommended. Both meet the owner's S-09 and S-10 rules. The revocation
semantics expose a real difference: B can honour them per op; A needs a replay and holds dependent honest edits aside.
S-06, S-08, alternative C and scoring come next. ADR-002 and ADR-003 stay `proposed`.
