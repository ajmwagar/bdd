#![feature(seek_convenience)]
use std::{thread, io::{stdin, stderr, stdout}, io::prelude::*, fs::File, path::PathBuf, error::Error};
use structopt::StructOpt;
use bus::Bus;

#[derive(StructOpt, Debug)]
#[structopt(name = "bd")]
/// A drop in, parallel dd
struct Opts {
    /// Input file to read from. if left empty STDIN is used.
    #[structopt(short = "i", long = "input", parse(from_os_str))]
    input: Option<PathBuf>,

    #[structopt(short = "o", long = "output")]
    /// Output File(s) to write to. If left empty STDOUT is used.
    output: Option<Vec<String>>,

    #[structopt(short = "b", long = "block-size", default_value = "64000")]
    block_size: usize,

    #[structopt(short = "b", long = "block-buffer", default_value = "20")]
    block_buffer: usize
}


fn main() -> Result<(), std::io::Error> {
    // Grap args
    let Opts { input, output, block_size, block_buffer } = Opts::from_args();

    let mut message_bus = Bus::new(block_buffer);

    let reader_thread: thread::JoinHandle<Result<usize, std::io::Error>> = thread::spawn(move || {
        match input {
            Some(input_path) => {
                let mut file = File::open(input_path)?;

                let mut read = 0;

                while file.stream_position()? < file.stream_len()? {
                    let mut tmp_buf = vec![0; block_size];

                    read += file.read(&mut tmp_buf)?;

                    message_bus.broadcast(tmp_buf);

                }

                Ok(read)
            }
            None => {
                let mut tmp_buf = vec![0; block_size];
                let mut stdin = stdin();

                let read = stdin.read_to_end(&mut tmp_buf);

                message_bus.broadcast(tmp_buf);

                read
            }
        }

    });

    let writer_threads: Vec<thread::JoinHandle<Result<usize, std::io::Error>>> = match output {
        Some(outputs) => {
            outputs.iter().map(|output_path| {
                let mut recv = message_bus.add_rx();
                thread::spawn(move || {
                    let mut file = File::open(&output_path)?;
                    
                    loop {
                        match recv.recv() {
                            Ok(bytes) => {
                                file.write(&bytes)?;
                            },
                            Err(_err) => {
                                // done writing
                                break;
                            }
                        }
                    }
                    
                    Ok(0)
                })
            }).collect()
        },
        // Use STDOUT
        None => {
            vec![thread::spawn(move || {
            let mut recv = message_bus.add_rx();
            let mut stdout = stdout();

            loop {
                match recv.recv() {
                    Ok(bytes) => {
                        stdout.write(&bytes)?;
                    },
                    Err(err) => {
                        // done writing
                        break;
                    }
                }
            }
            Ok(0)
            })]
            
        }
    };

    // Wait on threads
    let bytes_read = reader_thread.join().unwrap()?;

    println!("Read {} bytes.", bytes_read);

    writer_threads.iter().for_each(|handle| {
        handle.join();
    });

    Ok(())
}
