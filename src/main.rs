#![feature(seek_convenience)]
use bus::Bus;
use std::{
    fs::File,
    io::{prelude::*, stdin, stdout},
    path::PathBuf,
    process, thread,
};
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "bd")]
/// Bulk Data Duplicator
///
/// Simple interface to write image to many files/devices at once
/// Can also be used to backup to multiple locations
pub struct Opts {
    /// Input file to read from. if left empty STDIN is used.
    #[structopt(short = "i", long = "input", parse(from_os_str))]
    input: Option<PathBuf>,

    #[structopt(short = "o", long = "output", parse(from_os_str))]
    /// Output File(s) to write to. If left empty STDOUT is used.
    output: Option<Vec<PathBuf>>,

    #[structopt(short = "b", long = "block-size", default_value = "64000")]
    /// Set the block size to process data in
    block_size: usize,

    #[structopt(short = "f", long = "block-buffer", default_value = "20")]
    /// Set the amount of blocks to store in memory at a given time. memory usage = (block-buffer * # of output files * block-size)
    block_buffer: usize,

    #[structopt(short = "c", long = "count")]
    /// # of blocks to read, useful for generating random data from /dev/random or zeroing drives
    /// with /dev/zero
    block_count: Option<usize>,
}

impl Default for Opts {
    fn default() -> Self {
        Opts {
            input: None,
            output: None,
            block_size: 64000,
            block_buffer: 20,
            block_count: None
        }
    }
}

fn main() {
    match run(Opts::from_args()) {
        Ok(_) => process::exit(0),
        Err(err) => {
            eprintln!("Error: {}", err);
            process::exit(1)
        }
    }
}

pub fn run(opts: Opts) -> Result<(), std::io::Error> {
    let Opts {
        input,
        output,
        block_size,
        block_buffer,
        block_count,
    } = opts;
    let mut message_bus: Bus<Vec<u8>> = Bus::new(block_buffer);

    let writer_threads: Vec<thread::JoinHandle<Result<usize, std::io::Error>>> = match output {
        Some(outputs) => {
            outputs
                .into_iter()
                .map(|output_path| {
                    let mut recv = message_bus.add_rx();
                    thread::spawn(move || {
                        let mut file = File::create(&output_path)?;

                        loop {
                            match recv.recv() {
                                Ok(bytes) => {
                                    file.write_all(&bytes)?;
                                }
                                Err(_err) => {
                                    file.sync_all()?;
                                    // done writing
                                    break;
                                }
                            }
                        }

                        Ok(0)
                    })
                })
                .collect()
        }
        // Use STDOUT
        None => {
            let mut recv = message_bus.add_rx();
            vec![thread::spawn(move || {
                let mut stdout = stdout();

                loop {
                    match recv.recv() {
                        Ok(bytes) => {
                            stdout.write_all(&bytes)?;
                        }
                        Err(_err) => {
                            stdout.flush()?;
                            // done writing
                            break;
                        }
                    }
                }
                Ok(0)
            })]
        }
    };

    let reader_thread: thread::JoinHandle<Result<usize, std::io::Error>> =
        thread::spawn(move || match input {
            Some(input_path) => {
                let mut file = File::open(input_path)?;

                let mut read = 0;

                match block_count {
                    Some(count) => {
                        let mut counter = 0;
                        while counter < count {
                            let mut tmp_buf = vec![0; block_size];
                            read += file.read(&mut tmp_buf)?;
                            message_bus.broadcast(tmp_buf);

                            counter += 1;
                        }
                    }
                    None => {
                        while file.stream_position()? < file.stream_len()? {
                            let diff = (file.stream_len()? - file.stream_position()?) as usize;

                            let mut tmp_buf = if diff < block_size {
                                vec![0; diff]
                            } else {
                                vec![0; block_size]
                            };
                            read += file.read(&mut tmp_buf)?;
                            message_bus.broadcast(tmp_buf);
                        }
                    }
                };

                Ok(read)
            }
            None => {
                let mut tmp_buf = vec![0; block_size];
                let mut stdin = stdin();

                let read = stdin.read_to_end(&mut tmp_buf);

                message_bus.broadcast(tmp_buf);

                read
            }
        });

    // Wait on threads
    let bytes_read = reader_thread.join().unwrap()?;

    eprintln!(
        "{} bytes copied to {} files.",
        bytes_read,
        writer_threads.len()
    );

    for handle in writer_threads {
        handle.join().unwrap()?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{run, Opts};

    #[test]
    fn count_flag() {
        assert!(run(Opts {
            input: Some("/dev/urandom".into()),
            output: Some(vec!["/tmp/bd_test".into()]),
            block_count: Some(20),
            ..Opts::default()
        }).is_ok());
    }

    #[test]
    fn block_size_flag() {
        assert!(run(Opts {
            input: Some("./Cargo.toml".into()),
            output: Some(vec!["/tmp/bd_test".into()]),
            block_size: 200,
            ..Opts::default()
        }).is_ok());
    }

    #[test]
    fn multiple_outputs() {
        assert!(run(Opts {
            input: Some("./Cargo.toml".into()),
            output: Some(vec!["/tmp/bd_test".into(), "/tmp/bd_test_2".into(), "/tmp/bd_test_3".into()]),
            ..Opts::default()
        }).is_ok());
    }
}
