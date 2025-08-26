# vite_str

`vite_str::Str` is an owned string type that stores smaller strings on the stack.

Why introduce a new type instead of just using `CompactString`:

- There are lots of [string type implementations](https://github.com/rosetta-rs/string-rosetta-rs). We want to swap them easily if we found a better implementation for our usage.
- We need a string type that implements `bincode::{Encode, Decode}`. None of existing implementations has that.
