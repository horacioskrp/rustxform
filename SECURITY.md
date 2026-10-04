# Security Policy

## Supported versions

rustxform is pre-1.0; only the latest `0.2.x` release line receives security
fixes.

| Version | Supported |
| ------- | --------- |
| 0.2.x   | ✅        |
| < 0.2   | ❌        |

## Reporting a vulnerability

**Please do not open a public issue for security problems.**

Report privately through GitHub: on the repository's **Security** tab, choose
**Report a vulnerability** (GitHub Private Vulnerability Reporting). If that is
unavailable, contact the maintainer
[@horacioskrp](https://github.com/horacioskrp) and ask for a private channel.

Please include:

- affected version(s) and platform,
- a minimal XLSForm or XForm that reproduces the issue,
- the impact you observed (e.g. panic / crash, resource exhaustion, incorrect
  output that could mislead downstream data collection).

We aim to acknowledge a report within a few days, agree on a disclosure
timeline, and credit reporters who wish to be named once a fix is released.

## Scope & threat model

rustxform is a compiler library: it reads untrusted spreadsheets/XForms and
produces XForm XML. Security-relevant issues include memory-safety problems
(the workspace sets `unsafe_code = "forbid"`, so these should not occur),
panics or unbounded resource use on crafted input, and output that silently
diverges from the input's intent. The crate performs no network or file-system
access of its own beyond what the CLI is explicitly asked to read and write.
