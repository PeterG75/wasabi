use byteorder::{ReadBytesExt, WriteBytesExt};
use std::{i32, i64, io, u32};
use std::ops::Deref;

#[derive(Debug, PartialEq)]
pub struct Leb128<T> {
    pub value: T,
    /// When reading an `Leb128<T>`, the number of bytes used to encode the `value`.
    /// When writing an `Leb128<T>`, the minimum number of bytes used to encode `value`.
    ///
    /// This way, reading and writing an `Leb128<T>` always results in the same number of bytes,
    /// i.e., it round-trips.
    pub byte_count: usize, // TODO does this have to be pub?
}

impl<T> Leb128<T> {
    // TODO replace with static "with_byte_count" or something?
    /// Replace the value, but keep the byte_count from self.
    ///
    /// When encoding the resulting `Leb128<U>`, the byte_count will still be used to determine the
    /// minimum number of bytes for encoding U.
    pub fn map<U>(&self, new_value: U) -> Leb128<U> {
        Leb128 {
            value: new_value,
            byte_count: self.byte_count,
        }
    }
}

impl<T> Deref for Leb128<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

// TODO implement DerefMut?


pub trait ReadLeb128<T>: io::Read {
    fn read_leb128(&mut self) -> io::Result<Leb128<T>>;
}

pub trait WriteLeb128<T>: io::Write {
    /// Write LEB128 encoded `T`, using at least `value.byte_count` bytes.
    /// Returns the actual written byte count.
    fn write_leb128(&mut self, value: &Leb128<T>) -> io::Result<usize>;
}

// Need to write this as a macro, not a generic impl because
// a) num_traits are quite lacking, e.g., there is no "U as T" for primitive integers
// b) cannot use specialization: impl<T: PrimInt> for Leb128<T> overlaps (and is NOT more special
//    than) for example impl<T: Wasm> for Leb128<Vec<T>>
macro_rules! impl_leb128_integer {
    ($T:ident) => {
        impl<R: io::Read> ReadLeb128<$T> for R {
            fn read_leb128(&mut self) -> io::Result<Leb128<$T>> {
                let mut value = 0;
                let mut bytes_read = 0;
                let mut shift = 0;
                let mut byte = 0x80;

                while byte & 0x80 != 0 {
                    byte = self.read_u8()?;
                    // mask off continuation bit from byte and prepend lower 7 bits to value
                    if let Some(high_bits) = ((byte & 0x7f) as $T).checked_shl(shift) {
                        value |= high_bits;
                    } else {
                        return Err(io::Error::new(io::ErrorKind::InvalidData, format!("LEB128 to {} overflow", stringify!($T))));
                    }
                    bytes_read += 1;
                    shift += 7;
                }

                Ok(Leb128 {
                    value,
                    byte_count: bytes_read
                })
            }
        }

        impl<W: io::Write> WriteLeb128<$T> for W {
            fn write_leb128(&mut self, leb128: &Leb128<$T>) -> io::Result<usize> {
                let mut value = leb128.value;
                let mut bytes_written = 0;
                let mut more_bytes = true;

                while more_bytes {
                    // select low 7 bits of value
                    let mut byte_to_write = value as u8 & 0x7F;
                    // sign extends, important for signed integers!
                    value >>= 7;
                    bytes_written += 1;

                    // for unsigned integers, MIN and 0 are the same, but for signed ones the
                    // double check of value is important: -1 (all 1's) and 0 (all 0's) stop writing
                    more_bytes = (value > $T::MIN && value > 0) || bytes_written < leb128.byte_count;
                    if more_bytes {
                        byte_to_write |= 0x80;
                    }
                    self.write_u8(byte_to_write)?;
                }

                Ok(bytes_written)
            }
        }
    }
}

impl_leb128_integer!(u32);
impl_leb128_integer!(i32);
impl_leb128_integer!(i64);