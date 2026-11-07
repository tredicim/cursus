# Security policy

Cursus is local-first: your CV data stays on your machine and the tool makes no
network calls. The most relevant risks are therefore in how data files, paths
and LaTeX output are handled, and in the dependencies we ship.

## Supported versions

Cursus is pre-release. Only the latest commit on `main` receives security fixes;
there are no maintained release branches yet.

## Reporting a vulnerability

Please **do not open a public issue** for a security problem.

Report it privately to [info@saajansaini.com](mailto:info@saajansaini.com) with:

- a description of the issue and its impact;
- the affected commit or version;
- the steps or a minimal input needed to reproduce it;
- any suggested fix, if you have one.

If GitHub's private vulnerability reporting is enabled for this repository, you
can use the **Security** tab instead of email.

We aim to acknowledge reports within 7 days and to agree on a disclosure
timeline with you. There is no bug bounty.

## Scope

In scope:

- the `cursus` CLI and the core/render crates;
- parsing and validation of entry and target data;
- LaTeX escaping and the PDF build pipeline;
- the desktop app and its file handling;
- our dependency tree, when the issue is specific to Cursus.

Out of scope:

- vulnerabilities in LaTeX engines, browsers or operating systems;
- reports that require an already-compromised machine;
- spam or social engineering.

Please do not include real personal data (yours or anyone else's) in a report;
a minimal synthetic example is enough.
