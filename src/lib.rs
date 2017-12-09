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

use std::mem::transmute;
use std::io::Error;
use std::io::ErrorKind::{UnexpectedEof, WriteZero};

use futures::{Future, Poll};
use futures::Async::Ready;
use tokio_io::{AsyncRead, AsyncWrite};

macro_rules! gen_module {
    ($num:ty, $bytes:expr, $from_be:path, $from_le:path) => (
        use std::mem::transmute;
        use std::io::Error;
        use std::io::ErrorKind::{UnexpectedEof, WriteZero};

        use futures::{Future, Poll};
        use futures::Async::Ready;
        use tokio_io::{AsyncRead, AsyncWrite};
        
        /// Create a future to read a number in native byte order.
        pub fn read_native<R>(reader: R) -> ReadNative<R> {
            ReadNative {
                bytes: [0; $bytes],
                offset: 0,
                reader: Some(reader)
            }
        }
        
        /// Future to read a number in native byte order from an `AsyncRead`,
        /// created by the corresponding `read_xyz_native` function.
        pub struct ReadNative<R> {
            bytes: [u8; $bytes],
            offset: u8,
            reader: Option<R>
        }
        
        impl<R: AsyncRead> Future for ReadNative<R> {
            type Item = ($num, R);
            type Error = Error;

            /// Read a number in native byte order, retrying on `Interrupted`
            /// errors, and signaling `WouldBlock` errors via `Async::NotReady`.
            /// Returns an `UnexpectedEof` error if reading returns `Ok(0)`.
            fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
                let mut reader = self.reader
                    .take()
                    .expect("Polled reader after completion");

                while self.offset < $bytes {
                    match reader.read(&mut self.bytes[self.offset as usize..]) {
                        Ok(read) => {
                            if read == 0 {
                                return Err(Error::new(UnexpectedEof, "failed to read number"));
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

                Ok(Ready((unsafe { transmute::<[u8; $bytes], $num>(self.bytes) }, reader)))
            }
        }
        
        /// Create a future to read a big-endian number.
        pub fn read_be<R>(reader: R) -> ReadBE<R> {
            ReadBE(read_native(reader))
        }

        /// Future to read a big-endian number from an `AsyncRead`, created by
        /// the corresponding `read_xyz_be` function.
        pub struct ReadBE<R>(ReadNative<R>);

        impl<R: AsyncRead> Future for ReadBE<R> {
            type Item = ($num, R);
            type Error = Error;

            /// Read a big-endian number, retrying on `Interrupted` errors, and signaling
            /// `WouldBlock` errors via `Async::NotReady`. Returns an `UnexpectedEof`
            /// error if reading returns `Ok(0)`.
            fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
                let (num, reader) = try_ready!(self.0.poll());
                Ok(Ready(($from_be(num), reader)))
            }
        }
        
        /// Create a future to read a little-endian number.
        pub fn read_le<R>(reader: R) -> ReadLE<R> {
            ReadLE(read_native(reader))
        }

        /// Future to read a little-endian number from an `AsyncRead`, created by
        /// the corresponding `read_xyz_le` function.
        pub struct ReadLE<R>(ReadNative<R>);

        impl<R: AsyncRead> Future for ReadLE<R> {
            type Item = ($num, R);
            type Error = Error;

            /// Read a little-endian number, retrying on `Interrupted` errors, and signaling
            /// `WouldBlock` errors via `Async::NotReady`. Returns an `UnexpectedEof`
            /// error if reading returns `Ok(0)`.
            fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
                let (num, reader) = try_ready!(self.0.poll());
                Ok(Ready(($from_le(num), reader)))
            }
        }
        
        /// Create a future to write a number in native byte order.
        pub fn write_native<W>(num: $num, writer: W) -> WriteNative<W> {
            WriteNative {
                bytes: unsafe { transmute::<$num, [u8; $bytes]>(num) },
                offset: 0,
                writer: Some(writer),
            }
        }

        /// Future to write a number in native byte order to an `AsyncWrite`, created by
        /// the corresponding `write_xyz_native` function.
        pub struct WriteNative<W> {
            bytes: [u8; $bytes],
            offset: u8,
            writer: Option<W>,
        }


        impl<W: AsyncWrite> Future for WriteNative<W> {
            type Item = W;
            type Error = Error;

            /// Write a number in native byte order, retrying on `Interrupted`
            /// errors, and signaling `WouldBlock` errors via `Async::NotReady`.
            /// Returns a `WriteZero` error if writing returns `Ok(0)`.
            fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
                let mut writer = self.writer
                    .take()
                    .expect("Polled writer after completion");

                while self.offset < $bytes {
                    match writer.write(&mut self.bytes[self.offset as usize..]) {
                        Ok(written) => {
                            if written == 0 {
                                return Err(Error::new(WriteZero, "failed to write number"));
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

        /// Create a future to write a big-endian number.
        pub fn write_be<W>(num: $num, writer: W) -> WriteBE<W> {
            WriteBE(write_native(num.to_be(), writer))
        }

        /// Future to write a big-endian number to an `AsyncWrite`, created by
        /// the corresponding `write_xyz_be` function.
        pub struct WriteBE<W>(WriteNative<W>);

        impl<W: AsyncWrite> Future for WriteBE<W> {
            type Item = W;
            type Error = Error;

            /// Write a big-endian numer, retrying on `Interrupted` errors, and signaling
            /// `WouldBlock` errors via `Async::NotReady`. Returns a `WriteZero` error
            /// if writing returns `Ok(0)`.
            fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
                self.0.poll()
            }
        }

        /// Create a future to write a little-endian number.
        pub fn write_le<W>(num: $num, writer: W) -> WriteLE<W> {
            WriteLE(write_native(num.to_le(), writer))
        }

        /// Future to write a little-endian number to an `AsyncWrite`, created by
        /// the corresponding `write_le` function.
        pub struct WriteLE<W>(WriteNative<W>);

        impl<W: AsyncWrite> Future for WriteLE<W> {
            type Item = W;
            type Error = Error;

            /// Write a little-endian number, retrying on `Interrupted` errors, and signaling
            /// `WouldBlock` errors via `Async::NotReady`. Returns a `WriteZero` error
            /// if writing returns `Ok(0)`.
            fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
                self.0.poll()
            }
        }
    )
}

mod mod_u16 {
    gen_module!{u16, 2, u16::from_be, u16::from_le}
}
pub use self::mod_u16::read_native as read_u16_native;
pub use self::mod_u16::ReadNative as ReadU16Native;
pub use self::mod_u16::read_be as read_u16_be;
pub use self::mod_u16::ReadBE as ReadU16BE;
pub use self::mod_u16::read_le as read_u16_le;
pub use self::mod_u16::ReadLE as ReadU16LE;
pub use self::mod_u16::write_native as write_u16_native;
pub use self::mod_u16::WriteNative as WriteU16Native;
pub use self::mod_u16::write_be as write_u16_be;
pub use self::mod_u16::WriteBE as WriteU16BE;
pub use self::mod_u16::write_le as write_u16_le;
pub use self::mod_u16::WriteLE as WriteU16LE;

mod mod_u32 {
    gen_module!{u32, 4, u32::from_be, u32::from_le}
}
pub use self::mod_u32::read_native as read_u32_native;
pub use self::mod_u32::ReadNative as ReadU32Native;
pub use self::mod_u32::read_be as read_u32_be;
pub use self::mod_u32::ReadBE as ReadU32BE;
pub use self::mod_u32::read_le as read_u32_le;
pub use self::mod_u32::ReadLE as ReadU32LE;
pub use self::mod_u32::write_native as write_u32_native;
pub use self::mod_u32::WriteNative as WriteU32Native;
pub use self::mod_u32::write_be as write_u32_be;
pub use self::mod_u32::WriteBE as WriteU32BE;
pub use self::mod_u32::write_le as write_u32_le;
pub use self::mod_u32::WriteLE as WriteU32LE;

mod mod_u64 {
    gen_module!{u64, 8, u64::from_be, u64::from_le}
}
pub use self::mod_u64::read_native as read_u64_native;
pub use self::mod_u64::ReadNative as ReadU64Native;
pub use self::mod_u64::read_be as read_u64_be;
pub use self::mod_u64::ReadBE as ReadU64BE;
pub use self::mod_u64::read_le as read_u64_le;
pub use self::mod_u64::ReadLE as ReadU64LE;
pub use self::mod_u64::write_native as write_u64_native;
pub use self::mod_u64::WriteNative as WriteU64Native;
pub use self::mod_u64::write_be as write_u64_be;
pub use self::mod_u64::WriteBE as WriteU64BE;
pub use self::mod_u64::write_le as write_u64_le;
pub use self::mod_u64::WriteLE as WriteU64LE;

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

    macro_rules! gen_test {
        ($read_native:expr, $write_native:expr, $read_be:expr, $write_be:expr, $read_le:expr, $write_le:expr) => (
            #[test]
            fn test() {
                let rng = StdGen::new(rand::thread_rng(), 12);
                let mut quickcheck = QuickCheck::new().gen(rng).tests(10000);
                quickcheck.quickcheck(test_native as
                                      fn(usize,
                                         PartialWithErrors<GenInterruptedWouldBlock>,
                                         PartialWithErrors<GenInterruptedWouldBlock>)
                                         -> bool);
                                         
                 let rng = StdGen::new(rand::thread_rng(), 12);
                 let mut quickcheck = QuickCheck::new().gen(rng).tests(10000);
                 quickcheck.quickcheck(test_be as
                                       fn(usize,
                                          PartialWithErrors<GenInterruptedWouldBlock>,
                                          PartialWithErrors<GenInterruptedWouldBlock>)
                                          -> bool);
                                          
                  let rng = StdGen::new(rand::thread_rng(), 12);
                  let mut quickcheck = QuickCheck::new().gen(rng).tests(10000);
                  quickcheck.quickcheck(test_le as
                                        fn(usize,
                                           PartialWithErrors<GenInterruptedWouldBlock>,
                                           PartialWithErrors<GenInterruptedWouldBlock>)
                                           -> bool);
            }

            fn test_native(buf_size: usize,
                               write_ops: PartialWithErrors<GenInterruptedWouldBlock>,
                               read_ops: PartialWithErrors<GenInterruptedWouldBlock>)
                               -> bool {
                let num = 2;

                let (w, r) = ring_buffer(buf_size + 1);
                let mut w = PartialAsyncWrite::new(w, write_ops);
                let mut r = PartialAsyncRead::new(r, read_ops);
                let writer = $write_native(num, &mut w);
                let reader = $read_native(&mut r);

                let (_, (read, _)) = writer.join(reader).wait().unwrap();
                assert_eq!(read, num);

                return true;
            }
            
            fn test_be(buf_size: usize,
                               write_ops: PartialWithErrors<GenInterruptedWouldBlock>,
                               read_ops: PartialWithErrors<GenInterruptedWouldBlock>)
                               -> bool {
                let num = 2;

                let (w, r) = ring_buffer(buf_size + 1);
                let mut w = PartialAsyncWrite::new(w, write_ops);
                let mut r = PartialAsyncRead::new(r, read_ops);
                let writer = $write_be(num, &mut w);
                let reader = $read_be(&mut r);

                let (_, (read, _)) = writer.join(reader).wait().unwrap();
                assert_eq!(read, num);

                return true;
            }
            
            fn test_le(buf_size: usize,
                               write_ops: PartialWithErrors<GenInterruptedWouldBlock>,
                               read_ops: PartialWithErrors<GenInterruptedWouldBlock>)
                               -> bool {
                let num = 2;

                let (w, r) = ring_buffer(buf_size + 1);
                let mut w = PartialAsyncWrite::new(w, write_ops);
                let mut r = PartialAsyncRead::new(r, read_ops);
                let writer = $write_le(num, &mut w);
                let reader = $read_le(&mut r);

                let (_, (read, _)) = writer.join(reader).wait().unwrap();
                assert_eq!(read, num);

                return true;
            }
        )
    }

    mod test_u16 {
        use super::*;
        gen_test!{read_u16_native, write_u16_native, read_u16_be, write_u16_be, read_u16_le, write_u16_le}
    }

    mod test_u32 {
        use super::*;
        gen_test!{read_u32_native, write_u32_native, read_u32_be, write_u32_be, read_u32_le, write_u32_le}
    }

    mod test_u64 {
        use super::*;
        gen_test!{read_u64_native, write_u64_native, read_u64_be, write_u64_be, read_u64_le, write_u64_le}
    }
}
