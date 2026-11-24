# Security

## Reporting a problem

Report security problems privately, not in a public issue, pull request or
discussion. Use GitHub's private reporting: the **Security** tab of this
repository, then **Report a vulnerability**. Only the maintainers see the
report.

Please include what you ran, what happened and what you expected. If you attach
a script or other file to show the problem, remove anything personal from it
first. A small made-up script is best.

We aim to reply within seven days. We will tell you when a fix is ready and
credit you in the advisory and the changelog, unless you ask us not to.

## What counts

Anything that lets a file, an agent or a network peer do more than the user
asked for. For example:

- A command run through `wordsmith-cli mcp` reading or writing outside its
  `--read-root` or `--write-root` folders, or launching another program while
  confined.
- A script, Fountain, RTF, PDF or text file that crashes the app, hangs it, or
  makes it write somewhere unexpected when opened or imported.
- The update check or self-update installing anything that is not a verified
  release file.
- Release files that do not match the source they claim to be built from.
- Crash recovery, auto-save or backup files readable by other users.

Only the latest release is supported. Fixes are not backported.

## How we handle a report

Maintainers and contributors follow these steps for every security problem,
whether it was reported or found in a review:

1. **Keep it private until the fix ships.** Do not describe the problem in a
   public issue, pull request, discussion, commit message, branch name, code
   comment or file in the repository. Discuss it in the private advisory.
   Working notes stay out of the repository (see "Private working notes" in
   `CONTRIBUTING.md`).
2. **Fix it in the open, quietly.** The fix lands through a normal pull request.
   Its title and commit messages say what the code now does ("Exports go
   through the checked writer"), not how to exploit the old code. Add a test
   that would have caught it.
3. **Release.** Ship a release with the fix.
4. **Disclose.** Then publish the GitHub security advisory, and add a
   **Security** entry to `CHANGELOG.md` that names the problem, the versions
   affected and the version that fixes it.

If a problem is already public, or is being exploited, skip straight to a fix
and an advisory.
