//! Async encoders and decoders for the primitive number types, both big-endian and little-endian.
#![deny(missing_docs)]

extern crate async_codec;
extern crate async_codec_util;
extern crate atm_io_utils;
extern crate futures_core;
extern crate futures_io;


#[cfg(test)]
#[macro_use(quickcheck)]
extern crate quickcheck;
#[cfg(test)]
extern crate async_ringbuffer;

macro_rules! gen_byte_module {
    ($num:ty, $name:tt) => (
        use std::marker::PhantomData;

        use async_codec::{AsyncDecode, AsyncEncode, AsyncEncodeLen, PollEnc, PollDec};
        use futures_core::Never;
        use futures_core::Async::{Ready, Pending};
        use futures_core::task::Context;
        use futures_io::{AsyncRead, AsyncWrite, Error as FutIoErr, ErrorKind};

        #[doc = "Create a decoder for a `"]
        #[doc = $name]
        #[doc = "`."]
        pub fn decode_byte<R>() -> DecodeByte<R> {
            DecodeByte {_r: PhantomData}
        }

        #[doc = "Decode a `"]
        #[doc = $name]
        #[doc = "`."]
        pub struct DecodeByte<R> {
            _r: PhantomData<R>
        }

        impl<R: AsyncRead> AsyncDecode<R> for DecodeByte<R> {
            type Item = $num;
            type Error = Never;

            fn poll_decode(
                self,
                cx: &mut Context,
                reader: &mut R
            ) -> PollDec<Self::Item, Self,Self::Error> {
                let mut byte = [0];

                match reader.poll_read(cx, &mut byte) {
                    Ok(Ready(0)) => PollDec::Errored(FutIoErr::new(ErrorKind::UnexpectedEof, $name).into()),
                    Ok(Ready(_)) => PollDec::Done(byte[0] as $num, 1),
                    Ok(Pending) => PollDec::Pending(self),
                    Err(err) => PollDec::Errored(err.into())
                }
            }
        }

        #[doc = "Create an encoder for a `"]
        #[doc = $name]
        #[doc = "`."]
        pub fn encode_byte<W>(num: $num) -> EncodeByte<W> {
            EncodeByte {num: [num as u8], _w: PhantomData}
        }

        #[doc = "Encode a `"]
        #[doc = $name]
        #[doc = "`."]
        pub struct EncodeByte<W> {
            num: [u8; 1],
            _w: PhantomData<W>
        }

        impl<W: AsyncWrite> AsyncEncode<W> for EncodeByte<W> {
            fn poll_encode(
                self,
                cx: &mut Context,
                writer: &mut W
            ) -> PollEnc<Self> {
                match writer.poll_write(cx, &self.num[..]) {
                    Ok(Ready(0)) => PollEnc::Errored(FutIoErr::new(ErrorKind::WriteZero, $name).into()),
                    Ok(Ready(_)) => PollEnc::Done(1),
                    Ok(Pending) => PollEnc::Pending(self),
                    Err(err) => PollEnc::Errored(err)
                }
            }
        }

        impl<W: AsyncWrite> AsyncEncodeLen<W> for EncodeByte<W> {
            fn remaining_bytes(&self) -> usize {
                1
            }
        }
    )
}

macro_rules! gen_module {
    ($num:ty, $name:tt,  $bytes:expr, $from_be:path, $from_le:path) => (
        use std::mem::transmute;
        use std::marker::PhantomData;

        use async_codec::{AsyncDecode, AsyncEncode, AsyncEncodeLen, PollEnc, PollDec};
        use futures_core::Never;
        use futures_core::Async::{Ready, Pending};
        use futures_core::task::Context;
        use futures_io::{AsyncRead, AsyncWrite, Error as FutIoErr, ErrorKind};

        #[doc = "Create a decoder for a `"]
        #[doc = $name]
        #[doc = "` in native byte order."]
        pub fn decode_native<R>() -> DecodeNative<R> {
            DecodeNative {
                bytes: [0; $bytes],
                offset: 0,
                _r: PhantomData,
            }
        }

        #[doc = "Decode a `"]
        #[doc = $name]
        #[doc = "` in native byte order."]
        pub struct DecodeNative<R> {
            bytes: [u8; $bytes],
            offset: u8,
            _r: PhantomData<R>
        }

        impl<R: AsyncRead> AsyncDecode<R> for DecodeNative<R> {
            type Item = $num;
            type Error = Never;

            fn poll_decode(
                mut self,
                cx: &mut Context,
                reader: &mut R
            ) -> PollDec<Self::Item, Self, Self::Error> {
                match reader.poll_read(cx, &mut self.bytes[self.offset as usize..]) {
                    Ok(Ready(0)) => PollDec::Errored(FutIoErr::new(ErrorKind::UnexpectedEof, $name).into()),
                    Ok(Ready(read)) => {
                        self.offset += read as u8;

                        if self.offset < $bytes {
                            PollDec::Progress(self, read)
                        } else {
                            PollDec::Done(unsafe { transmute::<[u8; $bytes], $num>(self.bytes) }, read)
                        }
                    }
                    Ok(Pending) => PollDec::Pending(self),
                    Err(err) => PollDec::Errored(err.into())
                }
            }
        }

        #[doc = "Create a decoder for a `"]
        #[doc = $name]
        #[doc = "` in big-endian byte order."]
        pub fn decode_be<R>() -> DecodeBE<R> {
            DecodeBE(decode_native())
        }

        #[doc = "Decode a `"]
        #[doc = $name]
        #[doc = "` in big-endian byte order."]
        pub struct DecodeBE<R>(DecodeNative<R>);

        impl<R: AsyncRead> AsyncDecode<R> for DecodeBE<R> {
            type Item = $num;
            type Error = Never;

            fn poll_decode(
                self,
                cx: &mut Context,
                reader: &mut R
            ) -> PollDec<Self::Item, Self, Self::Error> {
                match self.0.poll_decode(cx, reader) {
                    PollDec::Done(item, read) => PollDec::Done($from_be(item), read),
                    PollDec::Progress(inner, read) => {
                        PollDec::Progress(DecodeBE(inner), read)
                    }
                    PollDec::Pending(inner) => {
                        PollDec::Pending(DecodeBE(inner))
                    }
                    PollDec::Errored(err) => PollDec::Errored(err),
                }
            }
        }

        #[doc = "Create a decoder for a `"]
        #[doc = $name]
        #[doc = "` in little-endian byte order."]
        pub fn decode_le<R>() -> DecodeLE<R> {
            DecodeLE(decode_native())
        }

        #[doc = "Decode a `"]
        #[doc = $name]
        #[doc = "` in little-endian byte order."]
        pub struct DecodeLE<R>(DecodeNative<R>);

        impl<R: AsyncRead> AsyncDecode<R> for DecodeLE<R> {
            type Item = $num;
            type Error = Never;

            fn poll_decode(
                self,
                cx: &mut Context,
                reader: &mut R
            ) -> PollDec<Self::Item, Self, Self::Error> {
                match self.0.poll_decode(cx, reader) {
                    PollDec::Done(item, read) => PollDec::Done($from_le(item), read),
                    PollDec::Progress(inner, read) => {
                        PollDec::Progress(DecodeLE(inner), read)
                    }
                    PollDec::Pending(inner) => {
                        PollDec::Pending(DecodeLE(inner))
                    }
                    PollDec::Errored(err) => PollDec::Errored(err),
                }
            }
        }

        #[doc = "Create an encoder for a `"]
        #[doc = $name]
        #[doc = "` in native byte order."]
        pub fn encode_native<W>(num: $num) -> EncodeNative<W> {
            EncodeNative {
                bytes: unsafe { transmute::<$num, [u8; $bytes]>(num) },
                offset: 0,
                _w: PhantomData,
            }
        }

        #[doc = "Encode a `"]
        #[doc = $name]
        #[doc = "` in native byte order."]
        pub struct EncodeNative<W> {
            bytes: [u8; $bytes],
            offset: u8,
            _w: PhantomData<W>
        }

        impl<W: AsyncWrite> AsyncEncode<W> for EncodeNative<W> {
            fn poll_encode(
                mut self,
                cx: &mut Context,
                writer: &mut W
            ) -> PollEnc<Self> {
                match writer.poll_write(cx, &mut self.bytes[self.offset as usize..]) {
                    Ok(Ready(0)) => PollEnc::Errored(FutIoErr::new(ErrorKind::WriteZero, $name).into()),
                    Ok(Ready(written)) => {
                        self.offset += written as u8;
                        if self.offset < $bytes {
                            PollEnc::Progress(self, written)
                        } else {
                            PollEnc::Done(written)
                        }
                    },
                    Ok(Pending) => PollEnc::Pending(self),
                    Err(err) => PollEnc::Errored(err)
                }
            }
        }

        impl<W: AsyncWrite> AsyncEncodeLen<W> for EncodeNative<W> {
            fn remaining_bytes(&self) -> usize {
                ($bytes - self.offset) as usize
            }
        }

        #[doc = "Create an encoder for a `"]
        #[doc = $name]
        #[doc = "` in big-endian byte order."]
        pub fn encode_be<W>(num: $num) -> EncodeBE<W> {
            EncodeBE(encode_native(num.to_be()))
        }

        #[doc = "Encode a `"]
        #[doc = $name]
        #[doc = "` in big-endian byte order."]
        pub struct EncodeBE<W>(EncodeNative<W>);

        impl<W: AsyncWrite> AsyncEncode<W> for EncodeBE<W> {
            fn poll_encode(
                self,
                cx: &mut Context,
                writer: &mut W
            ) -> PollEnc<Self> {
                match self.0.poll_encode(cx, writer) {
                    PollEnc::Done(written) => PollEnc::Done(written),
                    PollEnc::Progress(inner, written) => {
                        PollEnc::Progress(EncodeBE(inner), written)
                    }
                    PollEnc::Pending(inner) => {
                        PollEnc::Pending(EncodeBE(inner))
                    }
                    PollEnc::Errored(err) => PollEnc::Errored(err),
                }
            }
        }

        impl<W: AsyncWrite> AsyncEncodeLen<W> for EncodeBE<W> {
            fn remaining_bytes(&self) -> usize {
                self.0.remaining_bytes()
            }
        }

        #[doc = "Create an encoder for a `"]
        #[doc = $name]
        #[doc = "` in little-endian byte order."]
        pub fn encode_le<W>(num: $num) -> EncodeLE<W> {
            EncodeLE(encode_native(num.to_le()))
        }

        #[doc = "Encode a `"]
        #[doc = $name]
        #[doc = "` in little-endian byte order."]
        pub struct EncodeLE<W>(EncodeNative<W>);

        impl<W: AsyncWrite> AsyncEncode<W> for EncodeLE<W> {
            fn poll_encode(
                self,
                cx: &mut Context,
                writer: &mut W
            ) -> PollEnc<Self> {
                match self.0.poll_encode(cx, writer) {
                    PollEnc::Done(written) => PollEnc::Done(written),
                    PollEnc::Progress(inner, written) => {
                        PollEnc::Progress(EncodeLE(inner), written)
                    }
                    PollEnc::Pending(inner) => {
                        PollEnc::Pending(EncodeLE(inner))
                    }
                    PollEnc::Errored(err) => PollEnc::Errored(err),
                }
            }
        }

        impl<W: AsyncWrite> AsyncEncodeLen<W> for EncodeLE<W> {
            fn remaining_bytes(&self) -> usize {
                self.0.remaining_bytes()
            }
        }
    )
}

mod mod_u8 {
    gen_byte_module!{u8, "u8"}
}
pub use self::mod_u8::decode_byte as decode_u8;
pub use self::mod_u8::DecodeByte as DecodeU8;
pub use self::mod_u8::encode_byte as encode_u8;
pub use self::mod_u8::EncodeByte as EncodeU8;

mod mod_i8 {
    gen_byte_module!{i8, "i8"}
}
pub use self::mod_i8::decode_byte as decode_i8;
pub use self::mod_i8::DecodeByte as DecodeI8;
pub use self::mod_i8::encode_byte as encode_i8;
pub use self::mod_i8::EncodeByte as EncodeI8;

mod mod_u16 {
    gen_module!{u16, "u16", 2, u16::from_be, u16::from_le}
}
pub use self::mod_u16::decode_native as decode_u16_native;
pub use self::mod_u16::DecodeNative as DecodeU16Native;
pub use self::mod_u16::decode_be as decode_u16_be;
pub use self::mod_u16::DecodeBE as DecodeU16BE;
pub use self::mod_u16::decode_le as decode_u16_le;
pub use self::mod_u16::DecodeLE as DecodeU16LE;
pub use self::mod_u16::encode_native as encode_u16_native;
pub use self::mod_u16::EncodeNative as EncodeU16Native;
pub use self::mod_u16::encode_be as encode_u16_be;
pub use self::mod_u16::EncodeBE as EncodeU16BE;
pub use self::mod_u16::encode_le as encode_u16_le;
pub use self::mod_u16::EncodeLE as EncodeU16LE;

mod mod_u32 {
    gen_module!{u32, "u32", 4, u32::from_be, u32::from_le}
}
pub use self::mod_u32::decode_native as decode_u32_native;
pub use self::mod_u32::DecodeNative as DecodeU32Native;
pub use self::mod_u32::decode_be as decode_u32_be;
pub use self::mod_u32::DecodeBE as DecodeU32BE;
pub use self::mod_u32::decode_le as decode_u32_le;
pub use self::mod_u32::DecodeLE as DecodeU32LE;
pub use self::mod_u32::encode_native as encode_u32_native;
pub use self::mod_u32::EncodeNative as EncodeU32Native;
pub use self::mod_u32::encode_be as encode_u32_be;
pub use self::mod_u32::EncodeBE as EncodeU32BE;
pub use self::mod_u32::encode_le as encode_u32_le;
pub use self::mod_u32::EncodeLE as EncodeU32LE;

mod mod_u64 {
    gen_module!{u64, "u64", 8, u64::from_be, u64::from_le}
}
pub use self::mod_u64::decode_native as decode_u64_native;
pub use self::mod_u64::DecodeNative as DecodeU64Native;
pub use self::mod_u64::decode_be as decode_u64_be;
pub use self::mod_u64::DecodeBE as DecodeU64BE;
pub use self::mod_u64::decode_le as decode_u64_le;
pub use self::mod_u64::DecodeLE as DecodeU64LE;
pub use self::mod_u64::encode_native as encode_u64_native;
pub use self::mod_u64::EncodeNative as EncodeU64Native;
pub use self::mod_u64::encode_be as encode_u64_be;
pub use self::mod_u64::EncodeBE as EncodeU64BE;
pub use self::mod_u64::encode_le as encode_u64_le;
pub use self::mod_u64::EncodeLE as EncodeU64LE;

mod mod_i16 {
    gen_module!{i16, "i16", 2, i16::from_be, i16::from_le}
}
pub use self::mod_i16::decode_native as decode_i16_native;
pub use self::mod_i16::DecodeNative as DecodeI16Native;
pub use self::mod_i16::decode_be as decode_i16_be;
pub use self::mod_i16::DecodeBE as DecodeI16BE;
pub use self::mod_i16::decode_le as decode_i16_le;
pub use self::mod_i16::DecodeLE as DecodeI16LE;
pub use self::mod_i16::encode_native as encode_i16_native;
pub use self::mod_i16::EncodeNative as EncodeI16Native;
pub use self::mod_i16::encode_be as encode_i16_be;
pub use self::mod_i16::EncodeBE as EncodeI16BE;
pub use self::mod_i16::encode_le as encode_i16_le;
pub use self::mod_i16::EncodeLE as EncodeI16LE;

mod mod_i32 {
    gen_module!{i32, "i32", 4, i32::from_be, i32::from_le}
}
pub use self::mod_i32::decode_native as decode_i32_native;
pub use self::mod_i32::DecodeNative as DecodeI32Native;
pub use self::mod_i32::decode_be as decode_i32_be;
pub use self::mod_i32::DecodeBE as DecodeI32BE;
pub use self::mod_i32::decode_le as decode_i32_le;
pub use self::mod_i32::DecodeLE as DecodeI32LE;
pub use self::mod_i32::encode_native as encode_i32_native;
pub use self::mod_i32::EncodeNative as EncodeI32Native;
pub use self::mod_i32::encode_be as encode_i32_be;
pub use self::mod_i32::EncodeBE as EncodeI32BE;
pub use self::mod_i32::encode_le as encode_i32_le;
pub use self::mod_i32::EncodeLE as EncodeI32LE;

mod mod_i64 {
    gen_module!{i64, "i64", 8, i64::from_be, i64::from_le}
}
pub use self::mod_i64::decode_native as decode_i64_native;
pub use self::mod_i64::DecodeNative as DecodeI64Native;
pub use self::mod_i64::decode_be as decode_i64_be;
pub use self::mod_i64::DecodeBE as DecodeI64BE;
pub use self::mod_i64::decode_le as decode_i64_le;
pub use self::mod_i64::DecodeLE as DecodeI64LE;
pub use self::mod_i64::encode_native as encode_i64_native;
pub use self::mod_i64::EncodeNative as EncodeI64Native;
pub use self::mod_i64::encode_be as encode_i64_be;
pub use self::mod_i64::EncodeBE as EncodeI64BE;
pub use self::mod_i64::encode_le as encode_i64_le;
pub use self::mod_i64::EncodeLE as EncodeI64LE;

#[cfg(test)]
mod tests {
    use atm_io_utils::partial::*;
    use async_codec_util::testing::test_codec_len;
    use async_ringbuffer::ring_buffer;

    use super::*;

    macro_rules! gen_byte_test {
        ($num:ty, $decode_byte:expr, $encode_byte:expr) => (
            quickcheck! {
                fn test(buf_size: usize, read_ops: Vec<PartialOp>, write_ops: Vec<PartialOp>, num: $num) -> bool {
                    let mut read_ops = read_ops;
                    let mut write_ops = write_ops;
                    let (w, r) = ring_buffer(buf_size + 1);
                    let w = PartialWrite::new(w, write_ops.drain(..));
                    let r = PartialRead::new(r, read_ops.drain(..));

                    let test_outcome = test_codec_len(r, w, $decode_byte(), $encode_byte(num));
                    test_outcome.1 && test_outcome.0 == num
                }
            }
        );
    }

    macro_rules! gen_test {
        ($num: ty, $decode_native:expr, $encode_native:expr, $decode_be:expr, $encode_be:expr, $decode_le:expr, $encode_le:expr) => (
            quickcheck! {
                fn native(buf_size: usize, read_ops: Vec<PartialOp>, write_ops: Vec<PartialOp>, num: $num) -> bool {
                    let mut read_ops = read_ops;
                    let mut write_ops = write_ops;
                    let (w, r) = ring_buffer(buf_size + 1);
                    let w = PartialWrite::new(w, write_ops.drain(..));
                    let r = PartialRead::new(r, read_ops.drain(..));

                    let test_outcome = test_codec_len(r, w, $decode_native(), $encode_native(num));
                    test_outcome.1 && (test_outcome.0 == num)
                }
            }

            quickcheck! {
                fn be(buf_size: usize, read_ops: Vec<PartialOp>, write_ops: Vec<PartialOp>, num: $num) -> bool {
                    let mut read_ops = read_ops;
                    let mut write_ops = write_ops;
                    let (w, r) = ring_buffer(buf_size + 1);
                    let w = PartialWrite::new(w, write_ops.drain(..));
                    let r = PartialRead::new(r, read_ops.drain(..));

                    let test_outcome = test_codec_len(r, w, $decode_be(), $encode_be(num));
                    test_outcome.1 && test_outcome.0 == num
                }
            }

            quickcheck! {
                fn le(buf_size: usize, read_ops: Vec<PartialOp>, write_ops: Vec<PartialOp>, num: $num) -> bool {
                    let mut read_ops = read_ops;
                    let mut write_ops = write_ops;
                    let (w, r) = ring_buffer(buf_size + 1);
                    let w = PartialWrite::new(w, write_ops.drain(..));
                    let r = PartialRead::new(r, read_ops.drain(..));

                    let test_outcome = test_codec_len(r, w, $decode_le(), $encode_le(num));
                    test_outcome.1 && test_outcome.0 == num
                }
            }
        )
    }

    mod test_u8 {
        use super::*;
        gen_byte_test!{u8, decode_u8, encode_u8}
    }

    mod test_i8 {
        use super::*;
        gen_byte_test!{i8, decode_i8, encode_i8}
    }

    mod test_u16 {
        use super::*;
        gen_test!{u16, decode_u16_native, encode_u16_native, decode_u16_be, encode_u16_be, decode_u16_le, encode_u16_le}
    }

    mod test_u32 {
        use super::*;
        gen_test!{u32, decode_u32_native, encode_u32_native, decode_u32_be, encode_u32_be, decode_u32_le, encode_u32_le}
    }

    mod test_u64 {
        use super::*;
        gen_test!{u64, decode_u64_native, encode_u64_native, decode_u64_be, encode_u64_be, decode_u64_le, encode_u64_le}
    }

    mod test_i16 {
        use super::*;
        gen_test!{i16, decode_i16_native, encode_i16_native, decode_i16_be, encode_i16_be, decode_i16_le, encode_i16_le}
    }

    mod test_i32 {
        use super::*;
        gen_test!{i32, decode_i32_native, encode_i32_native, decode_i32_be, encode_i32_be, decode_i32_le, encode_i32_le}
    }

    mod test_i64 {
        use super::*;
        gen_test!{i64, decode_i64_native, encode_i64_native, decode_i64_be, encode_i64_be, decode_i64_le, encode_i64_le}
    }
}
