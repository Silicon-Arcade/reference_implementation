# Contributing

This repository is the reference implementation for the
book *6502 Construction Set*, and that purpose constrains how this repo
can accept contributions.

## What this repository is for

This is the finished emulator — where the book's thirty builds arrive, not a
snapshot of each step along the way. The book prints much of this code
verbatim, and what is here has to keep matching it. That constraint is the
point of the repository, and it means most improvements — better abstractions,
different module boundaries, more idiomatic Rust — cannot be accepted even when
they may be genuine improvements. A reader holding the book needs to find the code
the book shows them.

Similarly, the overarching goal for the Construction Set series of books is not for 
all readers to end up with the same outcome - and there are many places within the 
book where different design choices are considered - both explicitly and implicitly 
which also means 


If you want to take the design somewhere else, that is what the Apache license
is for. Fork it and build what you want. That is the intended use.

## What is welcome

**Issues, for anything.** Please open one for:

- a defect in the code — a wrong flag, a wrong cycle count, a test that passes
  when it should not
- errata in the book — a wrong address, a listing that does not assemble, a
  figure that contradicts the text
- a question about why something is the way it is
- a design preference you want to argue for, even knowing it will not be merged

A good report names the file or the page, says what you expected, and says what
happened instead. For processor behavior, a failing test is worth more than a
description of the problem.

I read everything that arrives. Replies may be slow while Book 2 is in
progress.

## Pull requests

I am not taking pull requests that change the design, for the reason above.

I will take, and am grateful for:

- a fix for a genuine defect, with a test that fails before it and passes after
- a typo or a broken link
- a fix for anything that does not build, or does not pass `cargo test`,
  `cargo clippy -- -D warnings` or `cargo fmt --check`

Keep a patch to one thing. A change I can read in a minute is far more likely
to land than a large one.

## Provenance

This project is clean-room: its behavior was established from datasheets,
documentation and observable results, never by reading another emulator's
source. Please do not send code derived from another emulator, and please say
if a contribution was generated or substantially written by an AI assistant.
Neither is automatically disqualifying, but both change what has to be verified
before it can go in, and the provenance of this code is something the book
makes claims about.

## Licensing

Contributions are under the Apache License 2.0, like the rest of the code.
Section 5 of the license covers this, so there is nothing separate to sign.
