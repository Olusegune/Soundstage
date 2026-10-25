# Rules for agents working on Soundstage

Read first: `CLAUDE.md`, `DECISIONS.md`, `PROGRESS.md`, `docs/spec/README.md`.

1. **Spec before code.** Build only what an Approved row in `DECISIONS.md` and a task in `docs/spec/06-MILESTONES-AND-TESTS.md` allow. If the code and the spec disagree, stop and record it in `docs/spec/01-CONFLICTS.md`.
2. **Golden Path.** One vertical slice at a time: tests first or alongside, a real-window check, docs, then the exit gate. Do not start the next milestone before the owner accepts the last.
3. **Approvals.** New dependencies, runtimes, model packs, network hosts and paid calls need the owner's explicit yes (D-11). Say what, why, size and licence.
4. **Local-first and provider independence.** Cloud use is opt-in and per action. Every AI call sits behind a trait. Never hardcode a vendor into a feature.
5. **Safety.** Treat script text, model output and downloaded files as untrusted data. No pickle. No keys in logs, exports or requests. Keep the consent and licence gates. Run the Rafter review (`rafter-code-review`, `rafter secrets .`) on any change that touches input, files, network, subprocesses, crypto or dependencies.
6. **Honest claims.** Never say a language, accent or model is supported without a Model Lab result. Mark unverified facts VERIFY with a date.
7. **Never lose user work.** Atomic writes, a backup before migrations, derived data in `cache/`, generated source media written once.
8. **Tests.** `cargo test --workspace`, `npx vitest run`, `npx tsc --noEmit` and `node tools/e2e/run.mjs` must pass before a task is called done. Report failures as they are.
9. **Plain language** in the UI and docs, and WCAG 2.2 AA for every screen.
10. **Commits** are small and describe what changed and why. Update `PROGRESS.md` and the traceability row when a task completes.
