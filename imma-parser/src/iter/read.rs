use std::io::Read;

use crate::parsers::{IMMAParseError, parse};
use crate::types::IMMARecord;

use bytes::{Bytes, BytesMut};

use super::FinishingIter;

const DEFAULT_BUFFER_CAPACITY: usize = 1024 * 100;
pub struct IMMAReadIterator<R> {
    inner: R,
    buffer: BytesMut,
    eof: bool,
    buffer_capacity: usize,
    error: Option<Error>,
}

pub struct IMMAReadIteratorBuilder<R> {
    inner: R,
    buffer_capacity: usize,
}

impl<R> IMMAReadIteratorBuilder<R>
where
    R: Read,
{
    pub fn with_buffer_capacity(mut self, capacity: usize) -> Self {
        self.buffer_capacity = capacity;
        self
    }

    pub fn new(reader: R) -> Self {
        Self {
            inner: reader,
            buffer_capacity: DEFAULT_BUFFER_CAPACITY,
        }
    }

    pub fn build(self) -> <Self as IntoIterator>::IntoIter {
        self.into_iter()
    }
}

impl<R> IntoIterator for IMMAReadIteratorBuilder<R>
where
    R: Read,
{
    type Item = <IMMAReadIterator<R> as Iterator>::Item;

    type IntoIter = IMMAReadIterator<R>;

    fn into_iter(self) -> Self::IntoIter {
        Self::IntoIter {
            inner: self.inner,
            buffer: BytesMut::with_capacity(self.buffer_capacity),
            eof: false,
            buffer_capacity: self.buffer_capacity,
            error: None,
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("parse error: {0}")]
    ParseError(#[from] nom::Err<crate::parsers::Error<Bytes>>),
    #[error("io error: {0}")]
    IoError(#[from] std::io::Error),
}

impl From<nom::Err<crate::parsers::Error<&[u8]>>> for Error {
    fn from(value: nom::Err<crate::parsers::Error<&[u8]>>) -> Self {
        let new_errors = match value {
            nom::Err::Incomplete(needed) => return Error::ParseError(nom::Err::Incomplete(needed)),
            nom::Err::Error(e) | nom::Err::Failure(e) => e.0.into_iter().map(|e| IMMAParseError {
                input: Bytes::copy_from_slice(e.input),
                context: e.context,
                kind: e.kind,
                other: e.other,
            }),
        }
        .collect();
        Error::ParseError(nom::Err::Error(new_errors))
    }
}

impl<R> IMMAReadIterator<R>
where
    R: Read,
{
    fn fill_buffer(&mut self) -> Result<usize, std::io::Error> {
        let scm = if self.buffer.spare_capacity_mut().is_empty() {
            self.buffer.reserve(self.buffer_capacity);
            unsafe {
                // SAFETY: Spare capacity is u8
                let scm = self.buffer.spare_capacity_mut();
                std::ptr::write_bytes(scm.as_mut_ptr() as *mut u8, 0, scm.len());
                scm
            }
        } else {
            self.buffer.spare_capacity_mut()
        };

        let read_len = unsafe {
            // SAFETY: The bytes have been reserved and initialized.
            let slice = std::slice::from_raw_parts_mut(scm.as_mut_ptr() as *mut u8, scm.len());
            let read_len = self.inner.read(slice)?;
            self.buffer.set_len(self.buffer.len() + read_len);
            read_len
        };

        Ok(read_len)
    }
}

impl<R> Iterator for IMMAReadIterator<R>
where
    R: Read,
{
    type Item = IMMARecord;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if (self.eof && self.buffer.is_empty()) || self.error.is_some() {
                return None;
            }
            let parse_result = parse(&self.buffer[..]);
            match parse_result {
                Ok((rem, Some(record))) => {
                    let consumed_len = self.buffer.len() - rem.len();
                    let _ = self.buffer.split_to(consumed_len);
                    return Some(record);
                }
                Ok((rem, None)) => {
                    let consumed_len = self.buffer.len() - rem.len();
                    let _ = self.buffer.split_to(consumed_len);
                }
                Err(e @ nom::Err::Incomplete(_)) => {
                    let e: Error = e.into();
                    match self.fill_buffer() {
                        Ok(0) => self.eof = true,
                        Ok(1..) => (),
                        Err(e) => {
                            self.error = Some(e.into());
                            return None;
                        }
                    }
                    if self.eof && !self.buffer.is_empty() {
                        self.error = Some(e);
                        return None;
                    }
                }
                Err(e @ (nom::Err::Error(_) | nom::Err::Failure(_))) => {
                    self.error = Some(e.into());
                    return None;
                }
            };
        }
    }
}

impl<R> FinishingIter for IMMAReadIterator<R> {
    type Error = Error;

    fn finish(self) -> Result<(), Self::Error> {
        match self.error {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }
}
