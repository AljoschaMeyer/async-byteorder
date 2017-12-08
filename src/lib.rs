#![warn(missing_docs)]
#[macro_use]
extern crate futures;
extern crate tokio_io;

#[cfg(test)]
extern crate quickcheck;
#[cfg(test)]
extern crate partial_io;
#[cfg(test)]
extern crate rand;
#[cfg(test)]
extern crate async_ringbuffer;
#[cfg(test)]
extern crate atm_io_utils;

use std::mem::transmute;
use std::io::Error;
use std::io::ErrorKind::{UnexpectedEof, WriteZero};

use futures::{Future, Poll};
use futures::Async::Ready;
use tokio_io::{AsyncRead, AsyncWrite};

/// Create a future to read a u32 in native byte order.
pub fn read_u32_native<R>(reader: R) -> ReadU32NativeOrder<R> {
    ReadU32NativeOrder::new(reader)
}

/// Future to read a `u32` in native byte order from an `AsyncRead`, created by
/// the `read_u32_native` function.
pub struct ReadU32NativeOrder<R> {
    bytes: [u8; 4],
    offset: u8,
    reader: Option<R>,
}

impl<R> ReadU32NativeOrder<R> {
    fn new(reader: R) -> ReadU32NativeOrder<R> {
        ReadU32NativeOrder {
            bytes: [0; 4],
            offset: 0,
            reader: Some(reader),
        }
    }
}

impl<R: AsyncRead> Future for ReadU32NativeOrder<R> {
    type Item = (u32, R);
    type Error = Error;

    /// Read a u32, retrying on `Interrupted` errors, and signaling
    /// `WouldBlock` errors via `Async::NotReady`. Returns an `UnexpectedEof`
    /// error if reading returns `Ok(0)`.
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let mut reader = self.reader
            .take()
            .expect("Polled reader after completion");

        while self.offset < 4 {
            match reader.read(&mut self.bytes[self.offset as usize..]) {
                Ok(read) => {
                    if read == 0 {
                        return Err(Error::new(UnexpectedEof, "failed to u32"));
                    }
                    self.offset += read as u8;
                }
                Err(ref e) if e.kind() == ::std::io::ErrorKind::WouldBlock => {
                    self.reader = Some(reader);
                    return Ok(::futures::Async::NotReady);
                }
                Err(ref e) if e.kind() == ::std::io::ErrorKind::Interrupted => {}
                Err(e) => return Err(e),
            }
        }

        Ok(Ready((unsafe { transmute::<[u8; 4], u32>(self.bytes) }, reader)))
    }
}

/// Create a future to read a big-endian u32.
pub fn read_u32_be<R>(reader: R) -> ReadU32BE<R> {
    ReadU32BE::new(reader)
}

/// Future to read a big-endian `u32` from an `AsyncRead`, created by the
/// `read_u32_be` function.
pub struct ReadU32BE<R>(ReadU32NativeOrder<R>);

impl<R> ReadU32BE<R> {
    fn new(reader: R) -> ReadU32BE<R> {
        ReadU32BE(ReadU32NativeOrder::new(reader))
    }
}

impl<R: AsyncRead> Future for ReadU32BE<R> {
    type Item = (u32, R);
    type Error = Error;

    /// Read a u32, retrying on `Interrupted` errors, and signaling
    /// `WouldBlock` errors via `Async::NotReady`. Returns an `UnexpectedEof`
    /// error if reading returns `Ok(0)`.
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let (num, reader) = try_ready!(self.0.poll());
        Ok(Ready((u32::from_be(num), reader)))
    }
}

/// Create a future to read a little-endian u32.
pub fn read_u32_le<R>(reader: R) -> ReadU32LE<R> {
    ReadU32LE::new(reader)
}

/// Future to read a little-endian `u32` from an `AsyncRead`, created by the
/// `read_u32_le` function.
pub struct ReadU32LE<R>(ReadU32NativeOrder<R>);

impl<R> ReadU32LE<R> {
    fn new(reader: R) -> ReadU32LE<R> {
        ReadU32LE(ReadU32NativeOrder::new(reader))
    }
}

impl<R: AsyncRead> Future for ReadU32LE<R> {
    type Item = (u32, R);
    type Error = Error;

    /// Read a u32, retrying on `Interrupted` errors, and signaling
    /// `WouldBlock` errors via `Async::NotReady`. Returns an `UnexpectedEof`
    /// error if reading returns `Ok(0)`.
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let (num, reader) = try_ready!(self.0.poll());
        Ok(Ready((u32::from_le(num), reader)))
    }
}

/// Create a future to write a u32 in native byte order.
pub fn write_u32_native<W>(num: u32, writer: W) -> WriteU32NativeOrder<W> {
    WriteU32NativeOrder::new(num, writer)
}

/// Future to write a `u32` in native byte order to an `AsyncWrite`, created by
/// the `write_u32_native` function.
pub struct WriteU32NativeOrder<W> {
    bytes: [u8; 4],
    offset: u8,
    writer: Option<W>,
}

impl<W> WriteU32NativeOrder<W> {
    fn new(num: u32, writer: W) -> WriteU32NativeOrder<W> {
        WriteU32NativeOrder {
            bytes: unsafe { transmute::<u32, [u8; 4]>(num) },
            offset: 0,
            writer: Some(writer),
        }
    }
}

impl<W: AsyncWrite> Future for WriteU32NativeOrder<W> {
    type Item = W;
    type Error = Error;

    /// Write a u32, retrying on `Interrupted` errors, and signaling
    /// `WouldBlock` errors via `Async::NotReady`. Returns a `WriteZero` error
    /// if writing returns `Ok(0)`.
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let mut writer = self.writer
            .take()
            .expect("Polled writer after completion");

        while self.offset < 4 {
            match writer.write(&mut self.bytes[self.offset as usize..]) {
                Ok(written) => {
                    if written == 0 {
                        return Err(Error::new(WriteZero, "failed to write u32"));
                    }
                    self.offset += written as u8;
                }
                Err(ref e) if e.kind() == ::std::io::ErrorKind::WouldBlock => {
                    self.writer = Some(writer);
                    return Ok(::futures::Async::NotReady);
                }
                Err(ref e) if e.kind() == ::std::io::ErrorKind::Interrupted => {}
                Err(e) => return Err(e),
            }
        }

        Ok(Ready(writer))
    }
}

/// Create a future to write a big-endian u32.
pub fn write_u32_be<W>(num: u32, writer: W) -> WriteU32BE<W> {
    WriteU32BE::new(num, writer)
}

/// Future to write a big-endian `u32` to an `AsyncWrite`, created by
/// the `write_u32_be` function.
pub struct WriteU32BE<W>(WriteU32NativeOrder<W>);

impl<W> WriteU32BE<W> {
    fn new(num: u32, writer: W) -> WriteU32BE<W> {
        WriteU32BE(WriteU32NativeOrder::new(num.to_be(), writer))
    }
}

impl<W: AsyncWrite> Future for WriteU32BE<W> {
    type Item = W;
    type Error = Error;

    /// Write a u32, retrying on `Interrupted` errors, and signaling
    /// `WouldBlock` errors via `Async::NotReady`. Returns a `WriteZero` error
    /// if writing returns `Ok(0)`.
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.0.poll()
    }
}

/// Create a future to write a little-endian u32.
pub fn write_u32_le<W>(num: u32, writer: W) -> WriteU32LE<W> {
    WriteU32LE::new(num, writer)
}

/// Future to write a little-endian `u32` to an `AsyncWrite`, created by
/// the `write_u32_le` function.
pub struct WriteU32LE<W>(WriteU32NativeOrder<W>);

impl<W> WriteU32LE<W> {
    fn new(num: u32, writer: W) -> WriteU32LE<W> {
        WriteU32LE(WriteU32NativeOrder::new(num.to_le(), writer))
    }
}

impl<W: AsyncWrite> Future for WriteU32LE<W> {
    type Item = W;
    type Error = Error;

    /// Write a u32, retrying on `Interrupted` errors, and signaling
    /// `WouldBlock` errors via `Async::NotReady`. Returns a `WriteZero` error
    /// if writing returns `Ok(0)`.
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.0.poll()
    }
}

#[cfg(test)]
mod tests {
    use futures::Future;
    use rand;
    use partial_io::{PartialAsyncRead, PartialAsyncWrite, PartialWithErrors};
    use partial_io::quickcheck_types::GenInterruptedWouldBlock;
    use quickcheck::{QuickCheck, StdGen};
    use async_ringbuffer::*;

    use super::*;

    use std::u32;

    #[test]
    fn test_u32() {
        let rng = StdGen::new(rand::thread_rng(), 8);
        let mut quickcheck = QuickCheck::new().gen(rng).tests(10000);
        quickcheck.quickcheck(test_u32_native as
                              fn(usize,
                                 PartialWithErrors<GenInterruptedWouldBlock>,
                                 PartialWithErrors<GenInterruptedWouldBlock>)
                                 -> bool);

        let rng = StdGen::new(rand::thread_rng(), 8);
        let mut quickcheck = QuickCheck::new().gen(rng).tests(10000);
        quickcheck.quickcheck(test_u32_be as
                              fn(usize,
                                 PartialWithErrors<GenInterruptedWouldBlock>,
                                 PartialWithErrors<GenInterruptedWouldBlock>)
                                 -> bool);

        let rng = StdGen::new(rand::thread_rng(), 8);
        let mut quickcheck = QuickCheck::new().gen(rng).tests(10000);
        quickcheck.quickcheck(test_u32_le as
                              fn(usize,
                                 PartialWithErrors<GenInterruptedWouldBlock>,
                                 PartialWithErrors<GenInterruptedWouldBlock>)
                                 -> bool);
    }

    fn test_u32_native(buf_size: usize,
                       write_ops: PartialWithErrors<GenInterruptedWouldBlock>,
                       read_ops: PartialWithErrors<GenInterruptedWouldBlock>)
                       -> bool {
        let num = u32::MAX - 1;

        let (w, r) = ring_buffer(buf_size + 1);
        let mut w = PartialAsyncWrite::new(w, write_ops);
        let mut r = PartialAsyncRead::new(r, read_ops);
        let writer = WriteU32NativeOrder::new(num, &mut w);
        let reader = ReadU32NativeOrder::new(&mut r);

        let (_, (read, _)) = writer.join(reader).wait().unwrap();
        assert_eq!(read, num);

        return true;
    }

    fn test_u32_be(buf_size: usize,
                   write_ops: PartialWithErrors<GenInterruptedWouldBlock>,
                   read_ops: PartialWithErrors<GenInterruptedWouldBlock>)
                   -> bool {
        let num = u32::MAX - 1;

        let (w, r) = ring_buffer(buf_size + 1);
        let mut w = PartialAsyncWrite::new(w, write_ops);
        let mut r = PartialAsyncRead::new(r, read_ops);
        let writer = WriteU32BE::new(num, &mut w);
        let reader = ReadU32BE::new(&mut r);

        let (_, (read, _)) = writer.join(reader).wait().unwrap();
        assert_eq!(read, num);

        return true;
    }

    fn test_u32_le(buf_size: usize,
                   write_ops: PartialWithErrors<GenInterruptedWouldBlock>,
                   read_ops: PartialWithErrors<GenInterruptedWouldBlock>)
                   -> bool {
        let num = u32::MAX - 1;

        let (w, r) = ring_buffer(buf_size + 1);
        let mut w = PartialAsyncWrite::new(w, write_ops);
        let mut r = PartialAsyncRead::new(r, read_ops);
        let writer = WriteU32LE::new(num, &mut w);
        let reader = ReadU32LE::new(&mut r);

        let (_, (read, _)) = writer.join(reader).wait().unwrap();
        assert_eq!(read, num);

        return true;
    }
}
