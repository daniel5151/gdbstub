# example_no_std

This basic example is used to benchmark how large `gdbstub`'s binary footprint is in `#![no_std]` contexts.

It uses many of the [`min-sized-rust`](https://github.com/johnthagen/min-sized-rust) guidelines to crunch down the binary size. This includes directly linking against `libc` to perform I/O, and avoiding and and all uses of Rust's [heavy formatting machinery](https://jamesmunns.com/blog/fmt-unreasonably-expensive/). While not perfect, this example should give a rough estimate of what a typical embedded system `gdbstub` integration might look like.

Oh, and please excuse the _terrible_ sockets code in `conn.rs`. I've never worked with raw C sockets, and that code was very haphazardly thrown together. If you're so inclined, I'd more than happily merge the PR that improves it's implementation!
