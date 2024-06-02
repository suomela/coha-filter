# coha-filter

Library for quickly finding data in the Corpus of Historical American English (COHA).

This assumes you have got a local copy of [Corpus of Historical American English - Kielipankki download version 2017H1](http://urn.fi/urn:nbn:fi:lb-2017061926). You only need to download and unzip it; no other preprocessing is needed.

## An example: BE *going to* V and *gonna*

In `examples/coha-be-going-to.rs` we have a sample program that searches for the following phrases in the entire COHA corpus:

- VB*, “going”, “to”, V?I*
- “gon”, “na”, *
- “gon”, “na”, V?I*

If your corpus is in e.g. `~/corpus/COHA/` and you would like to store the search results in `~/results/`, you can run it like this:

```sh
cargo run --release --example coha-be-going-to ~/corpus ~/results
```

You can enable more verbose logging with the usual `env_logger` environment variables, e.g.:

```sh
RUST_LOG=info cargo run --release --example coha-be-going-to ~/corpus ~/results
```

This should take less than half a minute; it will create CSV files in `~/results` that are organized by search term and decade. The files will contain the hit and 30 words of context on both sides.
