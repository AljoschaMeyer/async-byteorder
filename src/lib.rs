#![warn(missing_docs)]
#[macro_use]
extern crate futures;
extern crate tokio_io;
#[macro_use]
extern crate atm_io_utils;

#[cfg(test)]
extern crate quickcheck;
#[cfg(test)]
extern crate partial_io;
#[cfg(test)]
extern crate rand;
#[cfg(test)]
extern crate async_ringbuffer;

use std::mem::transmute;
use std::io::Error;
use std::io::ErrorKind::{UnexpectedEof, WriteZero};

use futures::{Future, Poll};
use futures::Async::Ready;
use tokio_io::{AsyncRead, AsyncWrite};

pub struct ReadU32NativeOrder<R> {
    bytes: [u8; 4],
    offset: u8,
    read: R,
}

impl<R> ReadU32NativeOrder<R> {
    pub fn new(read: R) -> ReadU32NativeOrder<R> {
        ReadU32NativeOrder {
            bytes: [0; 4],
            offset: 0,
            read,
        }
    }
}

impl<R: AsyncRead> Future for ReadU32NativeOrder<R> {
    type Item = u32;
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        while self.offset < 4 {
            let read = retry_nb!(self.read.read(&mut self.bytes[self.offset as usize..]));
            if read == 0 {
                return Err(Error::new(UnexpectedEof, "failed to u32"));
            }
            self.offset += read as u8;
        }

        Ok(Ready(unsafe { transmute::<[u8; 4], u32>(self.bytes) }))
    }
}

pub struct ReadU32BE<R>(ReadU32NativeOrder<R>);

impl<R> ReadU32BE<R> {
    pub fn new(read: R) -> ReadU32BE<R> {
        ReadU32BE(ReadU32NativeOrder::new(read))
    }
}

impl<R: AsyncRead> Future for ReadU32BE<R> {
    type Item = u32;
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        Ok(Ready(u32::from_be(try_ready!(self.0.poll()))))
    }
}

pub struct ReadU32LE<R>(ReadU32NativeOrder<R>);

impl<R> ReadU32LE<R> {
    pub fn new(read: R) -> ReadU32LE<R> {
        ReadU32LE(ReadU32NativeOrder::new(read))
    }
}

impl<R: AsyncRead> Future for ReadU32LE<R> {
    type Item = u32;
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        Ok(Ready(u32::from_le(try_ready!(self.0.poll()))))
    }
}

pub struct WriteU32NativeOrder<W> {
    bytes: [u8; 4],
    offset: u8,
    write: W,
}

impl<W> WriteU32NativeOrder<W> {
    pub fn new(num: u32, write: W) -> WriteU32NativeOrder<W> {
        WriteU32NativeOrder {
            bytes: unsafe { transmute::<u32, [u8; 4]>(num) },
            offset: 0,
            write,
        }
    }
}

impl<W: AsyncWrite> Future for WriteU32NativeOrder<W> {
    type Item = ();
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        while self.offset < 4 {
            let written = retry_nb!(self.write.write(&self.bytes[self.offset as usize..4]));
            if written == 0 {
                return Err(Error::new(WriteZero, "failed to write u32"));
            }
            self.offset += written as u8;
        }

        Ok(Ready(()))
    }
}

pub struct WriteU32BE<W>(WriteU32NativeOrder<W>);

impl<W> WriteU32BE<W> {
    pub fn new(num: u32, write: W) -> WriteU32BE<W> {
        WriteU32BE(WriteU32NativeOrder::new(num.to_be(), write))
    }
}

impl<W: AsyncWrite> Future for WriteU32BE<W> {
    type Item = ();
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.0.poll()
    }
}

pub struct WriteU32LE<W>(WriteU32NativeOrder<W>);

impl<W> WriteU32LE<W> {
    pub fn new(num: u32, write: W) -> WriteU32LE<W> {
        WriteU32LE(WriteU32NativeOrder::new(num.to_le(), write))
    }
}

impl<W: AsyncWrite> Future for WriteU32LE<W> {
    type Item = ();
    type Error = Error;

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
        let mut quickcheck = QuickCheck::new().gen(rng).tests(1000);
        quickcheck.quickcheck(test_u32_native as
                              fn(usize,
                                 PartialWithErrors<GenInterruptedWouldBlock>,
                                 PartialWithErrors<GenInterruptedWouldBlock>)
                                 -> bool);

        let rng = StdGen::new(rand::thread_rng(), 8);
        let mut quickcheck = QuickCheck::new().gen(rng).tests(1000);
        quickcheck.quickcheck(test_u32_be as
                              fn(usize,
                                 PartialWithErrors<GenInterruptedWouldBlock>,
                                 PartialWithErrors<GenInterruptedWouldBlock>)
                                 -> bool);

        let rng = StdGen::new(rand::thread_rng(), 8);
        let mut quickcheck = QuickCheck::new().gen(rng).tests(1000);
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

        let (_, read) = writer.join(reader).wait().unwrap();
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

        let (_, read) = writer.join(reader).wait().unwrap();
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

        let (_, read) = writer.join(reader).wait().unwrap();
        assert_eq!(read, num);

        return true;
    }
}
