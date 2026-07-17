use alloc::vec::Vec;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct BitBuffer {
    bytes: Vec<u8>,
    len:   usize,
}

impl BitBuffer {
    #[inline]
    pub(crate) fn with_capacity(bits: usize) -> Self {
        Self {
            bytes: Vec::with_capacity(bits.div_ceil(8)), len: 0
        }
    }

    #[inline]
    pub(crate) fn from_bytes(bytes: Vec<u8>) -> Self {
        let len = bytes.len() * 8;

        Self {
            bytes,
            len,
        }
    }

    #[cfg(any(feature = "qr", feature = "micro-qr", feature = "rmqr"))]
    #[inline]
    pub(crate) const fn len(&self) -> usize {
        self.len
    }

    #[inline]
    pub(crate) fn append(&mut self, value: u32, count: u8) {
        debug_assert!(count <= 32);
        debug_assert!(count == 32 || value >> count == 0);

        // QR Code fields are appended from the most significant selected bit.
        let mut remaining = usize::from(count);
        let used = self.len & 7;

        // The partially filled final byte takes as many leading bits as it can hold.
        if used != 0 {
            let take = (8 - used).min(remaining);

            remaining -= take;

            let bits = ((value >> remaining) as u8) & ((1 << take) - 1);
            let index = self.bytes.len() - 1;

            self.bytes[index] |= bits << (8 - used - take);
        }

        while remaining >= 8 {
            remaining -= 8;
            self.bytes.push((value >> remaining) as u8);
        }

        if remaining > 0 {
            self.bytes.push((value as u8 & ((1 << remaining) - 1)) << (8 - remaining));
        }

        self.len += usize::from(count);
    }

    #[cfg(any(feature = "qr", feature = "micro-qr", feature = "rmqr"))]
    pub(crate) fn extend(&mut self, other: &Self) {
        let whole_bytes = other.len >> 3;
        let trailing = (other.len & 7) as u8;

        if self.len & 7 == 0 {
            // A byte-aligned destination copies the whole source bytes directly.
            self.bytes.extend_from_slice(&other.bytes[..whole_bytes]);
            self.len += whole_bytes * 8;
        } else {
            for &byte in &other.bytes[..whole_bytes] {
                self.append(u32::from(byte), 8);
            }
        }

        if trailing != 0 {
            self.append(u32::from(other.bytes[whole_bytes]) >> (8 - trailing), trailing);
        }
    }

    #[cfg(any(feature = "qr", feature = "micro-qr", feature = "rmqr"))]
    #[inline]
    pub(crate) fn push(&mut self, bit: bool) {
        if self.len & 7 == 0 {
            self.bytes.push(0);
        }

        if bit {
            let index = self.bytes.len() - 1;
            self.bytes[index] |= 1 << (7 - (self.len & 7));
        }

        self.len += 1;
    }

    #[cfg(any(feature = "micro-qr", feature = "rmqr"))]
    #[inline]
    pub(crate) fn bit(&self, index: usize) -> bool {
        debug_assert!(index < self.len);

        self.bytes[index >> 3] >> (7 - (index & 7)) & 1 != 0
    }

    #[cfg(any(feature = "qr", feature = "rmqr"))]
    #[inline]
    pub(crate) fn into_bytes(self) -> Vec<u8> {
        debug_assert_eq!(self.len & 7, 0);

        self.bytes
    }

    #[cfg(feature = "micro-qr")]
    #[inline]
    pub(crate) fn into_padded_bytes(self) -> Vec<u8> {
        self.bytes
    }
}
