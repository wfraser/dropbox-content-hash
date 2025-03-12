# dropbox-content-hash

A utility & library for calculating Dropbox "content_hash" values.

Dropbox Content Hashes are the result of taking a file, dividing it into 4 MiB
blocks, calculating the SHA-256 hash of each block, concatenating the hashes,
and taking the SHA-256 hash of that.

Dropbox keeps a Content Hash of each file stored, which can be quickly obtained
through the API, and can be used to verify the integrity of the files uploaded
or downloaded from Dropbox.

See the [Dropbox Content Hash
Reference](https://www.dropbox.com/developers/reference/content-hash) for more
information.

This reposity contains a Rust crate for calculating the hashes, and a
command-line binary that runs that code on any given file. This implementation
uses `ring` for its cryptography implementation, which is extremely fast. In
addition, parallel hashing using threads can be enabled using an optional
feature `parallel` to make it even faster.

To build the command-line binary, the `cli` feature is required, so build using
`cargo build --release --features cli`

