extern crate dropbox_content_hash;
use dropbox_content_hash::*;

use std::fs::File;
use std::io::Read;
use std::process::exit;

fn usage() {
    eprintln!("usage: {} <file>", std::env::args().next().unwrap());
    eprintln!("    Calculate and print the Dropbox Content Hash of the given file.");
}

fn main() {
    let path = std::env::args_os()
        .nth(1)
        .unwrap_or_else(|| {
            usage();
            exit(1);
        });

    let mut f = File::open(&path)
        .unwrap_or_else(|e| {
            eprintln!("Failed to open {:?}: {}", path, e);
            exit(2);
        });

    let file_len = f.metadata()
        .map(|meta| meta.len())
        .ok(); // if we can't get file length, that's fine; just don't print progress

    let mut ctx = ContentHasher::new();
    let mut buf = vec![0u8; BLOCK_SIZE];
    let mut total_read = 0;
    loop {
        let nread = f.read(&mut buf)
            .unwrap_or_else(|e| {
                eprintln!("I/O error: {}", e);
                exit(2);
            });

        if nread == 0 {
            break;
        }
        ctx.update(&buf[0..nread]);
        total_read += nread;

        if let Some(len) = file_len {
            eprint!("\r{:.01}%", total_read as f64 / len as f64 * 100.);
        }
    }

    if file_len.is_some() {
        eprint!("\r");
    }

    println!("{}", ctx.finish_str());
}
