use clap::Parser;
use dropbox_content_hash::*;
use std::fs::File;
use std::io::{self, Read};
use std::path::PathBuf;
use std::process::exit;

/// Calculate and print the Dropbox Content Hash of the given file.
#[derive(Parser, Debug)]
#[command(version = format!("{}\nby {}", clap::crate_version!(), clap::crate_authors!(", ")))]
struct Args {
    /// Run the computation in parallel on the given number of threads.
    /// 0 means no parallelization at all.
    #[arg(long, default_value_t = 0)]
    threads: usize,

    /// Path to the file to hash.
    #[arg()]
    path: PathBuf,

    /// Print block hashes as well as the final hash.
    #[arg(long = "blocks")]
    print_block_hashes: bool,

    /// Disable printing of progress to the console.
    #[arg(long)]
    no_progress: bool,
}

fn main() {
    let args = Args::parse();

    let file = File::open(&args.path)
        .unwrap_or_else(|e| {
            eprintln!("Failed to open {:?}: {}", args.path, e);
            exit(2);
        });

    // If we want to print progress, we need to get the file length.
    // But printing progress interferes with parallel block hashes being printed, so skip it in
    // that case.
    let file_len = if !args.no_progress && (args.threads == 0 || !args.print_block_hashes) {
        file.metadata().map(|meta| meta.len()).ok()
    } else {
        None
    };

    let source: Box<dyn Read> = match file_len {
        Some(len) => Box::new(ProgressReader::new(file, len)),
        None      => Box::new(file),
    };

    if args.threads == 0 {
        let mut ctx = if args.print_block_hashes {
            ContentHasher::with_block_hashes_fn(Box::new(|block_num, hash| {
                println!("block {}: {}", block_num, dropbox_content_hash::hex_string(hash));
            }))
        } else {
            ContentHasher::default()
        };
        ctx.read_stream(source)
            .unwrap_or_else(|e| {
                eprintln!("I/O error: {}", e);
                exit(2);
            });
        println!("{}", ctx.finish_str());
    } else {
        match parallel::from_stream(
            source,
            args.threads,
            args.print_block_hashes.then_some(Box::new(|block_num, hash| {
                println!("block {}: {}", block_num, dropbox_content_hash::hex_string(hash));
            })))
        {
            Ok(hash) => {
                println!("{}", hex_string(&hash));
            }
            Err(e) => {
                eprintln!("{}", e);
                exit(2);
            }
        }
    }
}

struct ProgressReader<R> {
    inner: R,
    size: u64,
    position: u64,
}

impl<R> ProgressReader<R> {
    pub fn new(inner: R, size: u64) -> Self {
        Self {
            inner,
            size,
            position: 0,
        }
    }
}

impl<R: Read> Read for ProgressReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let nread = self.inner.read(buf)?;
        self.position += nread as u64;
        if nread == 0 {
            eprint!("      \r");
        } else {
            eprint!("{:.01}%\r", self.position as f64 / self.size as f64 * 100.);
        }
        Ok(nread)
    }
}
