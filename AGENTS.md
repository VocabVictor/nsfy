# nsfy — Agent rules

These apply to every sub-agent spawned in this repo. Break them, and the commit gets reverted.

## Git

- Author is always `VocabVictor <VocabVictor@users.noreply.github.com>`. Set it before committing.
- `git commit --author="VocabVictor <VocabVictor@users.noreply.github.com>"`
- Never add `Co-Authored-By`. Not for Claude, not for Copilot, not for anyone.
- Amend, don't stack fix commits. One logical change, one commit.
- Force push is fine on `main` — this is a solo repo.

## Code

- Read existing code before writing. Match naming, formatting, error handling style.
- Hand-written source files must not exceed 300 lines. Split by responsibility before crossing the limit. Generated files and dependency lockfiles are exempt.
- Run `pwsh -File scripts/check-source-lines.ps1` after source changes.
- No comments that restate the code. Comments explain why, not what.
- Log with `android.util.Log.i("nsfy", ...)` on Android, `info!()` on Rust server.
- No dead code, no `TODO` without a date and reason, no commented-out blocks.

## Building

- Rust: `cargo build --release` exits 0 before any commit to server/
- Desktop: `npm run build` exits 0 before any commit to desktop/src/
- Android: `./gradlew assembleRelease` exits 0 before any commit to android/
- If a build breaks, fix it in the same session. Don't leave master red.

## Scope

- Stay within `~/code/nsfy`. Don't touch files outside the project.
- Android SDK path: `~/android-sdk`
- Java home: `/opt/homebrew/opt/openjdk@17`
- No global installs. No `npm install -g`, no `cargo install`. Project-local only.

## Tone

- Plain English. Short sentences.
- No "Certainly!", "Great question", "I hope this helps", "feel free to reach out".
- No emoji in code, commit messages, or logs. README language flags are the exception.
