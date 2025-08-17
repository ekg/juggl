use clap::Parser;
use memmap2::MmapOptions;
use rayon::prelude::*;
use std::fs::File;
use std::io::{self, Write};
use std::num::NonZeroU32;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[derive(Parser, Debug)]
#[command(name = "juggl")]
#[command(about = "Shuffles chunks of a file based on delimiters", long_about = None)]
struct Args {
    #[arg(help = "Input file path")]
    input: PathBuf,

    #[arg(short, long, help = "Delimiter (supports escape sequences like \\x00)")]
    delimiter: String,

    #[arg(short, long, help = "Random seed for reproducible shuffling")]
    seed: Option<u64>,

    #[arg(short, long, help = "Number of threads for parallel processing (default: number of CPU cores)")]
    threads: Option<usize>,
}

fn parse_delimiter(delim: &str) -> Vec<u8> {
    let mut result = Vec::new();
    let chars: Vec<char> = delim.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '\\' && i + 1 < chars.len() {
            match chars[i + 1] {
                'n' => {
                    result.push(b'\n');
                    i += 2;
                }
                'r' => {
                    result.push(b'\r');
                    i += 2;
                }
                't' => {
                    result.push(b'\t');
                    i += 2;
                }
                '0' => {
                    result.push(b'\0');
                    i += 2;
                }
                'x' => {
                    if i + 3 < chars.len() {
                        let hex = &delim[i + 2..i + 4];
                        if let Ok(byte) = u8::from_str_radix(hex, 16) {
                            result.push(byte);
                            i += 4;
                        } else {
                            result.push(chars[i] as u8);
                            i += 1;
                        }
                    } else {
                        result.push(chars[i] as u8);
                        i += 1;
                    }
                }
                _ => {
                    result.push(chars[i + 1] as u8);
                    i += 2;
                }
            }
        } else {
            result.push(chars[i] as u8);
            i += 1;
        }
    }

    result
}

fn count_chunks_parallel(data: &[u8], delimiter: &[u8]) -> usize {
    if delimiter.is_empty() || data.is_empty() {
        return 1;
    }

    let delimiter = Arc::new(delimiter.to_vec());
    let data_len = data.len();
    let delim_len = delimiter.len();
    
    if data_len < delim_len {
        return 1;
    }
    
    let chunk_count = AtomicUsize::new(1); // Start with 1 for the initial chunk
    let chunk_size = std::cmp::max(1_000_000, data_len / rayon::current_num_threads());
    
    (0..data_len)
        .into_par_iter()
        .step_by(chunk_size)
        .for_each(|start| {
            let end = std::cmp::min(start + chunk_size + delim_len - 1, data_len);
            let delimiter = delimiter.clone();
            let mut local_count = 0;
            
            let mut i = start;
            while i <= end.saturating_sub(delim_len) {
                if &data[i..i + delim_len] == delimiter.as_slice() {
                    local_count += 1;
                    i += delim_len;
                } else {
                    i += 1;
                }
            }
            
            if local_count > 0 {
                chunk_count.fetch_add(local_count, Ordering::Relaxed);
            }
        });
    
    chunk_count.load(Ordering::Relaxed)
}

fn build_chunk_index(data: &[u8], delimiter: &[u8]) -> Vec<(usize, usize)> {
    let mut chunks = Vec::new();
    
    if delimiter.is_empty() || data.is_empty() {
        chunks.push((0, data.len()));
        return chunks;
    }

    let delim_len = delimiter.len();
    let mut chunk_start = 0;
    let mut i = 0;

    while i <= data.len().saturating_sub(delim_len) {
        if &data[i..i + delim_len] == delimiter {
            chunks.push((chunk_start, i));
            chunk_start = i + delim_len;
            i += delim_len;
        } else {
            i += 1;
        }
    }

    // Add the last chunk if there's remaining data
    if chunk_start < data.len() {
        chunks.push((chunk_start, data.len()));
    }

    chunks
}

fn main() -> io::Result<()> {
    let args = Args::parse();
    
    if let Some(threads) = args.threads {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build_global()
            .expect("Failed to set thread count");
    }
    
    let delimiter = parse_delimiter(&args.delimiter);
    
    let file = File::open(&args.input)?;
    let mmap = unsafe { MmapOptions::new().map(&file)? };
    
    // Pass 1: Count chunks in parallel
    let total_chunks = count_chunks_parallel(&mmap, &delimiter);
    
    if total_chunks == 0 {
        return Ok(());
    }
    
    // Pass 2: Build chunk index with a single scan
    let chunk_index = build_chunk_index(&mmap, &delimiter);
    
    // Generate permutation based on seed
    use hashed_permutation::HashedPermutation;
    let permutation = if let Some(seed) = args.seed {
        // Use the seed to create a deterministic permutation
        let seed_u32 = (seed & 0xFFFFFFFF) as u32;
        HashedPermutation {
            seed: seed_u32,
            length: NonZeroU32::new(chunk_index.len() as u32).unwrap(),
        }
    } else {
        // Random permutation
        use rand::Rng;
        let mut rng = rand::rng();
        let random_seed: u32 = rng.random();
        HashedPermutation {
            seed: random_seed,
            length: NonZeroU32::new(chunk_index.len() as u32).unwrap(),
        }
    };
    
    // Output chunks in permuted order
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    
    for i in 0..chunk_index.len() {
        // Get the permuted index for position i
        let permuted_idx = match permutation.shuffle(i as u32) {
            Ok(idx) => idx as usize,
            Err(_) => continue,
        };
        
        let (start, end) = chunk_index[permuted_idx];
        if start < end {
            let chunk_data = &mmap[start..end];
            
            // Write the chunk
            handle.write_all(chunk_data)?;
            
            // Add delimiter after chunk if not the last one
            if i < chunk_index.len() - 1 {
                handle.write_all(&delimiter)?;
            }
        }
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_delimiter_simple() {
        assert_eq!(parse_delimiter("abc"), vec![b'a', b'b', b'c']);
        assert_eq!(parse_delimiter(","), vec![b',']);
        assert_eq!(parse_delimiter("|"), vec![b'|']);
    }

    #[test]
    fn test_parse_delimiter_escape_sequences() {
        assert_eq!(parse_delimiter("\\n"), vec![b'\n']);
        assert_eq!(parse_delimiter("\\r"), vec![b'\r']);
        assert_eq!(parse_delimiter("\\t"), vec![b'\t']);
        assert_eq!(parse_delimiter("\\0"), vec![b'\0']);
    }

    #[test]
    fn test_parse_delimiter_hex() {
        assert_eq!(parse_delimiter("\\x00"), vec![0x00]);
        assert_eq!(parse_delimiter("\\x0a"), vec![0x0a]);
        assert_eq!(parse_delimiter("\\x41"), vec![0x41]);
        assert_eq!(parse_delimiter("\\xff"), vec![0xff]);
    }

    #[test]
    fn test_parse_delimiter_mixed() {
        assert_eq!(parse_delimiter("a\\nb"), vec![b'a', b'\n', b'b']);
        assert_eq!(parse_delimiter("\\x00,\\x01"), vec![0x00, b',', 0x01]);
    }

    #[test]
    fn test_parse_delimiter_invalid_hex() {
        assert_eq!(parse_delimiter("\\xgg"), vec![b'\\', b'x', b'g', b'g']);
        assert_eq!(parse_delimiter("\\x1"), vec![b'\\', b'x', b'1']);
    }

    #[test]
    fn test_parse_delimiter_escaped_backslash() {
        assert_eq!(parse_delimiter("\\\\"), vec![b'\\']);
        assert_eq!(parse_delimiter("\\a"), vec![b'a']);
    }

    #[test]
    fn test_count_chunks() {
        let data = b"a,b,c,d";
        let delimiter = b",";
        assert_eq!(count_chunks_parallel(data, delimiter), 4);
    }

    #[test]
    fn test_count_chunks_empty() {
        let data = b"";
        let delimiter = b",";
        assert_eq!(count_chunks_parallel(data, delimiter), 1);
    }

    #[test]
    fn test_count_chunks_no_delimiter() {
        let data = b"abcd";
        let delimiter = b",";
        assert_eq!(count_chunks_parallel(data, delimiter), 1);
    }

    #[test]
    fn test_build_chunk_index() {
        let data = b"a,b,c,d";
        let delimiter = b",";
        let index = build_chunk_index(data, delimiter);
        
        assert_eq!(index, vec![(0, 1), (2, 3), (4, 5), (6, 7)]);
    }

    #[test]
    fn test_build_chunk_index_with_consecutive_delimiters() {
        let data = b"a,,b";
        let delimiter = b",";
        let index = build_chunk_index(data, delimiter);
        
        assert_eq!(index, vec![(0, 1), (2, 2), (3, 4)]);
    }
}