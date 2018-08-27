//! Utility to calculate Dropbox Content Hashes.
//! 
//! Dropbox Content Hashes are the result of taking a file, dividing it into 4 MiB blocks,
//! calculating a SHA-256 hash of each block, concatenating the hashes, and taking the SHA-256 of
//! that.
//!
//! Dropbox keeps a Content Hash of each file stored, which can be quickly obtained through the
//! API, and can be used to verify the integrity of files uploaded to or downloaded from Dropbox.

extern crate ring;

use ring::digest::Context as HashContext;
use ring::digest::SHA256;

use std::cell::Cell;
use std::io::{self, Read};

/// The size of a Dropbox block: 4 MiB.
pub const BLOCK_SIZE: usize = 4 * 1024 * 1024;

/// The size of the resulting content hash: 256 bits.
pub const HASH_OUTPUT_SIZE: usize = 256 / 8;

/// A 
pub struct ContentHasher {
    ctx: HashContext,
    block_ctx: Cell<HashContext>,
    partial: usize,
}

impl ContentHasher {
    /// Create a new, empty, hasher.
    pub fn new() -> ContentHasher {
        ContentHasher {
            ctx: HashContext::new(&SHA256),
            block_ctx: Cell::new(HashContext::new(&SHA256)),
            partial: 0,
        }
    }

    /// Convenience function to hash an arbitrary byte stream.
    pub fn from_stream<R: Read>(mut r: R) -> io::Result<ContentHasher> {
        let mut ctx = ContentHasher::new();
        let mut buf = Vec::with_capacity(BLOCK_SIZE);
        buf.resize(BLOCK_SIZE, 0u8);
        loop {
            let nread = r.read(&mut buf)?;
            ctx.update(&buf[0..nread]);
            if nread == 0 {
                break;
            }
        }
        Ok(ctx)
    }

    fn finish_block(&mut self) {
        let block_ctx = self.block_ctx.replace(HashContext::new(&SHA256));
        self.ctx.update(block_ctx.finish().as_ref());
        self.partial = 0;
    }

    /// Update the content hash with some data.
    pub fn update(&mut self, bytes: &[u8]) {
        // First, finish off any partial block.
        let bytes = if self.partial != 0 {
            let (first, remaining) = bytes.split_at(BLOCK_SIZE - self.partial);
            self.block_ctx.get_mut().update(first);
            if self.partial + first.len() == BLOCK_SIZE {
                self.finish_block();
            }
            remaining
        } else {
            bytes
        };

        for block in bytes.chunks(BLOCK_SIZE) {
            self.block_ctx.get_mut().update(block);
            if block.len() < BLOCK_SIZE {
                // last block in this update
                self.partial = block.len();
            } else {
                self.finish_block();
            }
        }
    }

    /// Finish the content hash and return the bytes.
    pub fn finish(mut self) -> [u8; HASH_OUTPUT_SIZE] {
        if self.partial != 0 {
            self.ctx.update(self.block_ctx.into_inner().finish().as_ref());
        }
        let mut out = [0u8; HASH_OUTPUT_SIZE];
        out.copy_from_slice(self.ctx.finish().as_ref());
        out
    }

    /// Finish the content hash and return it as a hexadecimal string.
    pub fn finish_str(self) -> String {
        let bytes = self.finish();
        bytes.iter()
            .fold(String::new(),
                |s, byte| s + &format!("{:02x}", byte))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_bytes() {
        let ctx1 = ContentHasher::new();
        let r1 = ctx1.finish_str();

        let mut ctx2 = ContentHasher::new();
        ctx2.update(&[]);
        let r2 = ctx2.finish_str();

        assert_eq!(&r1, &r2);
        assert_eq!("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855", &r1);
    }

    #[test]
    fn less_than_one_block() {
        let mut ctx = ContentHasher::new();
        ctx.update(b"hello");
        assert_eq!(5, ctx.partial);
        assert_eq!(
            "9595c9df90075148eb06860365df33584b75bff782a510c6cd4883a419833d50",
            &ctx.finish_str());
    }

    #[test]
    fn exactly_one_block() {
        let mut ctx = ContentHasher::new();
        ctx.update(&[30; BLOCK_SIZE]);
        assert_eq!(0, ctx.partial);
        assert_eq!(
            "1114501b241325c24970e0cd0b6416d80284085151e2980747ccecc4e0c156e6",
            &ctx.finish_str());
    }

    #[test]
    fn one_block_and_a_little_bit_more() {
        let mut ctx = ContentHasher::new();
        ctx.update(&[30; BLOCK_SIZE + 1]);
        assert_eq!(1, ctx.partial);
        assert_eq!(
            "5b1d15f99119b9138a887c27d1b246cf6c584621fc75c42edd27c3d962835d4f",
            &ctx.finish_str());
    }

    #[test]
    fn exactly_two_blocks() {
        let mut ctx = ContentHasher::new();
        ctx.update(&[30; 2 * BLOCK_SIZE]);
        assert_eq!(0, ctx.partial);
        assert_eq!(
            "aa562efb265c604214e4626717330e15be16f2daaabfe5d7d2c22f3e88cbc268",
            &ctx.finish_str());
    }

    #[test]
    fn exactly_two_blocks_separately() {
        let mut ctx = ContentHasher::new();
        ctx.update(&[30; BLOCK_SIZE]);
        ctx.update(&[30; BLOCK_SIZE]);
        assert_eq!(
            "aa562efb265c604214e4626717330e15be16f2daaabfe5d7d2c22f3e88cbc268",
            &ctx.finish_str());
    }

    #[test]
    fn partial_blocks() {
        let mut ctx = ContentHasher::new();
        ctx.update(&[30; BLOCK_SIZE / 2]);
        ctx.update(&[30; BLOCK_SIZE]);
        ctx.update(&[30; BLOCK_SIZE / 2]);
        assert_eq!(
            "aa562efb265c604214e4626717330e15be16f2daaabfe5d7d2c22f3e88cbc268",
            &ctx.finish_str());
    }
}
