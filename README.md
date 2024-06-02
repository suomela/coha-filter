# coha-filter

Library for quickly finding data in the Corpus of Historical American English (COHA).

This assumes you have got a local copy of COHA on your own computer; we have used [Corpus of Historical American English - Kielipankki download version 2017H1](http://urn.fi/urn:nbn:fi:lb-2017061926).

The program will read the corpus files that were provided in the [relational database format](https://www.corpusdata.org/database.asp). You do not need to do any preprocessing, and you do not need to have a relational database. The program will just read the text files as such.

## An example: BE *going to* V and *gonna*

In `examples/coha-be-going-to.rs` we have a sample program that searches for the following phrases in the entire COHA corpus:

- VB*, “going”, “to”, V?I*
- “gon”, “na”, *
- “gon”, “na”, V?I*

If your corpus is in e.g. `~/COHA/` and you would like to store the search results in `~/results/`, you can run it like this:

```sh
cargo run --release --example coha-be-going-to ~/COHA ~/results
```

This should take less than half a minute; it will create CSV files in `~/results` that are organized by search term and decade. The files will contain the hit and 30 words of context on both sides.

## Author

[Jukka Suomela](https://jukkasuomela.fi)

## Acknowledgements

This was developed in collaboration with [Tanja Säily](https://tanjasaily.fi) and [Florent Perek](http://www.fperek.net).
