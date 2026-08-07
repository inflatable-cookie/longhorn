# Activation Source Adapters

Date: 2026-08-07
Card: 156
Roadmap: g02.010

## Result

`ActivationSource` lets a consuming application put any backend behind
licence acquisition and inherit evaluation unchanged. Two reference adapters
ship; hosted services stay consumer-implemented, with a worked example in
the tests rather than a shipped integration.

## Shape

`acquire`, `accept`, `renew`, `release`. Adapters describe exchanges; the
host performs them — the same posture as contract 018's `UpdateSource`, so a
consumer who has integrated the updater recognises the shape.

`Activation` distinguishes three outcomes: settled locally, done, and needs
an exchange. That is what lets a signed licence file take no network path at
all while redemption takes one, through one interface.

Defaults carry weight. `renew` settles unchanged and `release` returns done,
which are the correct answers for a source holding no lease or slot.
`accept` refuses, because a source that never requests an exchange should
never be handed a response.

## Decisions

**Release is in the interface, not left to consumers.** Contract 019
requires self-service release, and an interface that cannot express it
guarantees every "I got a new laptop" reaches a human. It is the dominant
licensing support ticket, so the answer belongs in the shape.

**`asserted_remotely` is named, not incidental.** Backends returning their
own response shape need a way to produce a licence without a signature. A
consumer reaching for that function is choosing a weaker offline guarantee,
and the type says so rather than letting it pass unnoticed.

**Licence keys accept what people actually type.** Crockford base32, grouped
in fives, position-weighted check symbol mod 37. Lower case, missing dashes,
whitespace, and the I/L→1 and O→0 confusions all parse — rejecting those
would be rejecting the customer for the typeface's mistake.

The weighting is load-bearing: an unweighted sum accepts any reordering of
the same symbols, and transposition is one of the two mistakes people make.
Tested directly.

`is_probably_a_typo` distinguishes a mistyped key from malformed input, so a
surface can say "check that key" instead of implying the key is worthless.
The check symbol is explicitly not a security feature and does not need to
be — a forged key that passes it still fails redemption. Its whole job is to
avoid a round trip that answers "invalid key" and leaves the customer
believing they were sold a dud.

**`ActivationUrl` is HTTPS-only, with no loopback exception.** Unlike an
update artifact, nothing here is third-party signature-verified end to end,
and an activation request carries credentials.

**`ActivationUrl` duplicates `longhorn-update`'s `EndpointUrl` deliberately.**
Both are optional capability crates; coupling them so one cannot be composed
without the other would cost more than thirty lines of validation. Recorded
as a papercut — promote to a shared primitive if a third caller appears.

## Evidence

11 activation tests, 48 in the crate. The ones that matter: a licence file
settles with no network and yields an offline-verifiable basis; redemption
describes an exchange rather than performing one; release and renew carry
the activation slot; a `HostedServiceSource` written in the test file
declares a remote assertion and inherits the weaker grace with no extra
wiring anywhere.

`cargo fmt --check` clean, clippy clean on both feature passes, full
workspace suite green.

## Notes

Two check-symbol expectations in the first draft were my guesses rather than
computed values; the tests caught both immediately. The tests now assert
against `from_body` output, so the issuing and parsing sides cannot drift.
