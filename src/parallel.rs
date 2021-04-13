//! Compute a content hash from a file or other stream, using multiple threads.

use crate::{BLOCK_SIZE, HASH_OUTPUT_SIZE};
use parallel_reader::read_stream_and_process_chunks_in_parallel;
use ring::digest::{digest, Context, Digest, SHA256};
use std::collections::BTreeMap;
use std::convert::TryInto;
use std::io::{self, Read};
use std::sync::{Arc, Mutex};

struct State {
    blocks: BTreeMap<u64, Digest>,
    next_offset: u64,
    overall_hash: Context,
    incomplete_block_offset: Option<u64>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            blocks: BTreeMap::new(),
            next_offset: 0,
            overall_hash: Context::new(&SHA256),
            incomplete_block_offset: None,
        }
    }
}

impl State {
    /// Add a finished block to the internal hash buffer and update the overall hash if possible.
    pub fn add_block(&mut self, block_hash: Digest, offset: u64) {
        if offset == self.next_offset {
            // shortcut: skip adding to the block map and add it directly
            self.incorporate_next_block(block_hash);
        } else {
            self.blocks.insert(offset, block_hash);
        }
        self.update_overall_hash();
    }

    /// Consume sequential blocks in the internal hash buffer and update the overall hash.
    fn update_overall_hash(&mut self) {
        loop {
            let key = self.next_offset;
            if let Some(hash) = self.blocks.remove(&key) {
                self.incorporate_next_block(hash);
            } else {
                break;
            }
        }
    }

    /// Add a block to the overall hash and update the next offset pointer.
    fn incorporate_next_block(&mut self, hash: Digest) {
        println!("block: {}", crate::hex_string(hash.as_ref()));
        self.overall_hash.update(hash.as_ref());
        self.next_offset += BLOCK_SIZE as u64;
    }

    /// Return the finished content hash.
    pub fn finish(mut self) -> Digest {
        if !self.blocks.is_empty() {
            self.update_overall_hash();
        }
        assert!(self.blocks.is_empty(), "all blocks must be incorporated in the overall hash at \
            this point");
        self.overall_hash.finish()
    }
}

/// Compute a content hash from the given file or other stream, using the specified number of
/// threads to do the computation in parallel.
pub fn content_hash_from_stream(
    source: impl Read,
    num_threads: usize,
) -> io::Result<[u8; HASH_OUTPUT_SIZE]> {

    let state = Arc::new(Mutex::new(State::default()));
    let thread_state = state.clone();
    match read_stream_and_process_chunks_in_parallel(source, BLOCK_SIZE, num_threads,
        Arc::new(move |offset, data: &[u8]| -> Result<(), u64> {
            let block_hash = digest(&SHA256, data);
            let mut state = thread_state.lock().unwrap();

            // Only the last block in the stream can be smaller than the full block size.
            if let Some(other_offset) = state.incomplete_block_offset {
                // Check where the other one is; if it's after this, it might be okay because it
                // might be the last block in the stream.
                if other_offset < offset {
                    return Err(other_offset);
                }
            }
            if data.len() != BLOCK_SIZE {
                if let Some(other_offset) = state.incomplete_block_offset {
                    return Err(offset.min(other_offset));
                }
                state.incomplete_block_offset = Some(offset);
            }

            state.add_block(block_hash, offset);

            Ok(())
        }),
    ) {
        Ok(()) => (),
        Err(parallel_reader::Error::Read(io_err)) => return Err(io_err),
        Err(parallel_reader::Error::Process { chunk_offset: _, error: bad_offset }) => {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                format!("incomplete block mid-stream at offset {:#x}", bad_offset)));
        }
    }

    // No other thread can have a copy of the Arc now, so extract the State out of the Arc and
    // Mutex so we can call finish().
    let state = Arc::try_unwrap(state)
        .map_err(|_| "unable to unpack state").unwrap()  // can't expect() because it's not Debug.
        .into_inner().unwrap();

    let digest = state.finish();

    Ok(digest.as_ref().try_into().expect("hash output is of wrong size"))
}
