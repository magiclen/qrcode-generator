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
        for shift in (0..count).rev() {
            self.push(((value >> shift) & 1) != 0);
        }
    }

    #[cfg(any(feature = "qr", feature = "micro-qr", feature = "rmqr"))]
    #[inline]
    pub(crate) fn extend(&mut self, other: &Self) {
        for index in 0..other.len {
            self.push(other.bit(index));
        }
    }

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

    #[cfg(any(feature = "qr", feature = "micro-qr", feature = "rmqr"))]
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
