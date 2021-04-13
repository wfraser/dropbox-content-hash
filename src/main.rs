use dropbox_content_hash::*;
use std::fs::File;
use std::io::{self, Read};
use std::path::PathBuf;
use std::process::exit;
use structopt::StructOpt;

/// Calculate and print the Dropbox Content Hash of the given file.
#[derive(StructOpt)]
struct Args {
    /// If specified, run the computation in parallel on the given number of threads.
    #[structopt(long)]
    threads: Option<usize>,

    /// Path to the file to hash.
    #[structopt(parse(from_os_str))]
    path: PathBuf,
}

fn main() {
    let args = Args::from_args();

    let file = File::open(&args.path)
        .unwrap_or_else(|e| {
            eprintln!("Failed to open {:?}: {}", args.path, e);
            exit(2);
        });

    let file_len = file.metadata()
        .map(|meta| meta.len())
        .ok(); // if we can't get file length, that's fine; just don't print progress

    let source: Box<dyn Read> = match file_len {
        Some(len) => Box::new(ProgressReader::new(file, len)),
        None      => Box::new(file),
    };

    match args.threads {
        None | Some(0) => {
            let ctx = ContentHasher::from_stream(source)
                .unwrap_or_else(|e| {
                    eprintln!("I/O error: {}", e);
                    exit(2);
                });
            println!("{}", ctx.finish_str());
        }
        Some(num_threads) => {
            match parallel::content_hash_from_stream(source, num_threads) {
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
