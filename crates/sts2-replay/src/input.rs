//! Optional gzip transport, independent of the text grammar and state decoding.

use std::io::{self, BufRead, BufReader, Chain, Cursor, Read};

use flate2::read::MultiGzDecoder;

type Prefixed<R> = Chain<Cursor<Vec<u8>>, BufReader<R>>;

enum Input<R: Read> {
    Plain(Prefixed<R>),
    Gzip(MultiGzDecoder<Prefixed<R>>),
}

impl<R: Read> Read for Input<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            Self::Plain(input) => input.read(buf),
            Self::Gzip(input) => input.read(buf),
        }
    }
}

/// Detects gzip by magic bytes and streams decoded text without a temporary file.
/// Plain input, short reads, and concatenated gzip members are supported. Reading
/// to EOF verifies gzip trailers; damaged or truncated members return I/O errors.
pub fn decode_input<R: Read>(input: R) -> io::Result<impl BufRead> {
    let mut input = BufReader::new(input);
    let mut prefix = Vec::with_capacity(2);
    input.by_ref().take(2).read_to_end(&mut prefix)?;
    let gzip = prefix == [0x1f, 0x8b];
    let input = Cursor::new(prefix).chain(input);
    Ok(BufReader::new(if gzip {
        Input::Gzip(MultiGzDecoder::new(input))
    } else {
        Input::Plain(input)
    }))
}
