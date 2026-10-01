#[cfg(any(feature = "qr", feature = "rmqr"))]
use alloc::borrow::Cow;
#[cfg(feature = "qr")]
use alloc::collections::BTreeMap;
#[cfg(feature = "qr")]
use alloc::vec;
use alloc::{string::String, vec::Vec};
#[cfg(any(feature = "qr", feature = "micro-qr", feature = "rmqr"))]
use core::ops::RangeInclusive;

mod bits;
#[cfg(any(feature = "qr", feature = "micro-qr", feature = "rmqr"))]
mod reed_solomon;

#[cfg(feature = "micro-qr")]
mod micro;
#[cfg(feature = "qr")]
mod model2;
#[cfg(any(feature = "qr", feature = "rmqr"))]
mod optimizer;
#[cfg(feature = "rmqr")]
mod rmqr;

use bits::BitBuffer;

use crate::EncodeError;

#[cfg(any(feature = "qr", feature = "micro-qr", feature = "rmqr"))]
#[inline]
const fn numeric_character_capacity(capacity_bits: usize, overhead_bits: usize) -> usize {
    let payload_bits = capacity_bits.saturating_sub(overhead_bits);

    payload_bits / 10 * 3
        + match payload_bits % 10 {
            4..=6 => 1,
            7..=9 => 2,
            _ => 0,
        }
}

#[cfg(any(feature = "qr", feature = "micro-qr", feature = "rmqr"))]
#[inline]
const fn ensure_input_length(
    length: usize,
    per_symbol_capacity: usize,
    capacity_bits: usize,
    symbol_count: usize,
) -> Result<(), EncodeError> {
    if length > per_symbol_capacity.saturating_mul(symbol_count) {
        Err(EncodeError::DataTooLong {
            required_bits: None,
            capacity_bits: capacity_bits.saturating_mul(symbol_count),
        })
    } else {
        Ok(())
    }
}

/// The error correction level written to an encoded symbol.
///
/// The available variants depend on the enabled symbol family features.
#[cfg(any(feature = "qr", feature = "micro-qr", feature = "rmqr"))]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum SymbolErrorCorrection {
    /// Detects errors in a Micro QR M1 symbol without correcting them.
    #[cfg(feature = "micro-qr")]
    DetectionOnly,
    /// Uses the Low error correction level.
    Low,
    /// Uses the Medium error correction level.
    Medium,
    /// Uses the Quartile error correction level.
    Quartile,
    /// Uses the High error correction level.
    #[cfg(any(feature = "qr", feature = "rmqr"))]
    High,
}

/// An error correction level for a Model 2 QR Code.
#[cfg(feature = "qr")]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum QrErrorCorrection {
    /// Uses the Low error correction level.
    Low,
    /// Uses the Medium error correction level.
    Medium,
    /// Uses the Quartile error correction level.
    Quartile,
    /// Uses the High error correction level.
    High,
}

#[cfg(feature = "qr")]
impl QrErrorCorrection {
    #[inline]
    pub(crate) const fn qr_ordinal(self) -> usize {
        match self {
            Self::Low => 0,
            Self::Medium => 1,
            Self::Quartile => 2,
            Self::High => 3,
        }
    }

    #[inline]
    pub(crate) const fn qr_format_bits(self) -> u8 {
        match self {
            Self::Low => 1,
            Self::Medium => 0,
            Self::Quartile => 3,
            Self::High => 2,
        }
    }
}

#[cfg(feature = "qr")]
impl From<QrErrorCorrection> for SymbolErrorCorrection {
    #[inline]
    fn from(value: QrErrorCorrection) -> Self {
        match value {
            QrErrorCorrection::Low => Self::Low,
            QrErrorCorrection::Medium => Self::Medium,
            QrErrorCorrection::Quartile => Self::Quartile,
            QrErrorCorrection::High => Self::High,
        }
    }
}

/// An error correction level for a Micro QR Code.
#[cfg(feature = "micro-qr")]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum MicroErrorCorrection {
    /// Detects errors in an M1 symbol without correcting them.
    DetectionOnly,
    /// Uses the Low error correction level.
    Low,
    /// Uses the Medium error correction level.
    Medium,
    /// Uses the Quartile error correction level.
    Quartile,
}

#[cfg(feature = "micro-qr")]
impl From<MicroErrorCorrection> for SymbolErrorCorrection {
    #[inline]
    fn from(value: MicroErrorCorrection) -> Self {
        match value {
            MicroErrorCorrection::DetectionOnly => Self::DetectionOnly,
            MicroErrorCorrection::Low => Self::Low,
            MicroErrorCorrection::Medium => Self::Medium,
            MicroErrorCorrection::Quartile => Self::Quartile,
        }
    }
}

/// An error correction level for a Rectangular Micro QR Code.
#[cfg(feature = "rmqr")]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RmqrErrorCorrection {
    /// Uses the Medium error correction level.
    Medium,
    /// Uses the High error correction level.
    High,
}

#[cfg(feature = "rmqr")]
impl From<RmqrErrorCorrection> for SymbolErrorCorrection {
    #[inline]
    fn from(value: RmqrErrorCorrection) -> Self {
        match value {
            RmqrErrorCorrection::Medium => Self::Medium,
            RmqrErrorCorrection::High => Self::High,
        }
    }
}

/// A validated Model 2 QR Code version.
#[cfg(feature = "qr")]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct QrVersion(u8);

#[cfg(feature = "qr")]
impl QrVersion {
    /// The largest Model 2 QR Code version.
    pub const MAX: Self = Self(40);
    /// The smallest Model 2 QR Code version.
    pub const MIN: Self = Self(1);

    /// Creates a Model 2 version from a value in `1..=40`.
    #[inline]
    pub const fn new(value: u8) -> Result<Self, EncodeError> {
        if value >= 1 && value <= 40 {
            Ok(Self(value))
        } else {
            Err(EncodeError::InvalidVersionRange)
        }
    }

    /// Returns the numeric version value.
    #[inline]
    pub const fn value(self) -> u8 {
        self.0
    }
}

/// A Micro QR Code version.
#[cfg(feature = "micro-qr")]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum MicroVersion {
    /// An 11 by 11 Micro QR Code symbol.
    M1,
    /// A 13 by 13 Micro QR Code symbol.
    M2,
    /// A 15 by 15 Micro QR Code symbol.
    M3,
    /// A 17 by 17 Micro QR Code symbol.
    M4,
}

#[cfg(feature = "micro-qr")]
impl MicroVersion {
    /// Returns the number of modules on each side.
    #[inline]
    pub const fn size(self) -> usize {
        match self {
            Self::M1 => 11,
            Self::M2 => 13,
            Self::M3 => 15,
            Self::M4 => 17,
        }
    }
}

/// A Rectangular Micro QR Code version.
#[cfg(feature = "rmqr")]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum RmqrVersion {
    /// A 7 by 43 rMQR symbol.
    R7x43,
    /// A 7 by 59 rMQR symbol.
    R7x59,
    /// A 7 by 77 rMQR symbol.
    R7x77,
    /// A 7 by 99 rMQR symbol.
    R7x99,
    /// A 7 by 139 rMQR symbol.
    R7x139,
    /// A 9 by 43 rMQR symbol.
    R9x43,
    /// A 9 by 59 rMQR symbol.
    R9x59,
    /// A 9 by 77 rMQR symbol.
    R9x77,
    /// A 9 by 99 rMQR symbol.
    R9x99,
    /// A 9 by 139 rMQR symbol.
    R9x139,
    /// An 11 by 27 rMQR symbol.
    R11x27,
    /// An 11 by 43 rMQR symbol.
    R11x43,
    /// An 11 by 59 rMQR symbol.
    R11x59,
    /// An 11 by 77 rMQR symbol.
    R11x77,
    /// An 11 by 99 rMQR symbol.
    R11x99,
    /// An 11 by 139 rMQR symbol.
    R11x139,
    /// A 13 by 27 rMQR symbol.
    R13x27,
    /// A 13 by 43 rMQR symbol.
    R13x43,
    /// A 13 by 59 rMQR symbol.
    R13x59,
    /// A 13 by 77 rMQR symbol.
    R13x77,
    /// A 13 by 99 rMQR symbol.
    R13x99,
    /// A 13 by 139 rMQR symbol.
    R13x139,
    /// A 15 by 43 rMQR symbol.
    R15x43,
    /// A 15 by 59 rMQR symbol.
    R15x59,
    /// A 15 by 77 rMQR symbol.
    R15x77,
    /// A 15 by 99 rMQR symbol.
    R15x99,
    /// A 15 by 139 rMQR symbol.
    R15x139,
    /// A 17 by 43 rMQR symbol.
    R17x43,
    /// A 17 by 59 rMQR symbol.
    R17x59,
    /// A 17 by 77 rMQR symbol.
    R17x77,
    /// A 17 by 99 rMQR symbol.
    R17x99,
    /// A 17 by 139 rMQR symbol.
    R17x139,
}

#[cfg(feature = "rmqr")]
impl RmqrVersion {
    pub(crate) const ALL: [Self; 32] = [
        Self::R7x43,
        Self::R7x59,
        Self::R7x77,
        Self::R7x99,
        Self::R7x139,
        Self::R9x43,
        Self::R9x59,
        Self::R9x77,
        Self::R9x99,
        Self::R9x139,
        Self::R11x27,
        Self::R11x43,
        Self::R11x59,
        Self::R11x77,
        Self::R11x99,
        Self::R11x139,
        Self::R13x27,
        Self::R13x43,
        Self::R13x59,
        Self::R13x77,
        Self::R13x99,
        Self::R13x139,
        Self::R15x43,
        Self::R15x59,
        Self::R15x77,
        Self::R15x99,
        Self::R15x139,
        Self::R17x43,
        Self::R17x59,
        Self::R17x77,
        Self::R17x99,
        Self::R17x139,
    ];
    // Versions ordered by area, then height, then width; sorted once at compile time for candidate search.
    pub(crate) const ALL_BY_AREA: [Self; 32] = Self::sorted_by_area();
    /// The last rMQR version in format indicator order.
    pub const MAX: Self = Self::R17x139;
    /// The first rMQR version in format indicator order.
    pub const MIN: Self = Self::R7x43;

    const fn sorted_by_area() -> [Self; 32] {
        let mut versions = Self::ALL;
        let length = versions.len();
        let mut index = 0;

        // A selection sort is enough for a const array of thirty-two versions.
        while index < length {
            let mut smallest = index;
            let mut candidate = index + 1;

            while candidate < length {
                if versions[candidate].area_key() < versions[smallest].area_key() {
                    smallest = candidate;
                }

                candidate += 1;
            }

            let swap = versions[index];
            versions[index] = versions[smallest];
            versions[smallest] = swap;
            index += 1;
        }

        versions
    }

    // Packs area, height and width into one number that orders the same way as the (area, height, width) tuple.
    const fn area_key(self) -> u64 {
        let width = self.width() as u64;
        let height = self.height() as u64;

        (width * height) * 1_000_000 + height * 1_000 + width
    }

    /// Returns the five-bit version indicator value.
    #[inline]
    pub const fn value(self) -> u8 {
        self as u8
    }

    /// Returns the symbol width in modules.
    #[inline]
    pub const fn width(self) -> usize {
        match self {
            Self::R11x27 | Self::R13x27 => 27,
            Self::R7x43
            | Self::R9x43
            | Self::R11x43
            | Self::R13x43
            | Self::R15x43
            | Self::R17x43 => 43,
            Self::R7x59
            | Self::R9x59
            | Self::R11x59
            | Self::R13x59
            | Self::R15x59
            | Self::R17x59 => 59,
            Self::R7x77
            | Self::R9x77
            | Self::R11x77
            | Self::R13x77
            | Self::R15x77
            | Self::R17x77 => 77,
            Self::R7x99
            | Self::R9x99
            | Self::R11x99
            | Self::R13x99
            | Self::R15x99
            | Self::R17x99 => 99,
            Self::R7x139
            | Self::R9x139
            | Self::R11x139
            | Self::R13x139
            | Self::R15x139
            | Self::R17x139 => 139,
        }
    }

    /// Returns the symbol height in modules.
    #[inline]
    pub const fn height(self) -> usize {
        match self {
            Self::R7x43 | Self::R7x59 | Self::R7x77 | Self::R7x99 | Self::R7x139 => 7,
            Self::R9x43 | Self::R9x59 | Self::R9x77 | Self::R9x99 | Self::R9x139 => 9,
            Self::R11x27
            | Self::R11x43
            | Self::R11x59
            | Self::R11x77
            | Self::R11x99
            | Self::R11x139 => 11,
            Self::R13x27
            | Self::R13x43
            | Self::R13x59
            | Self::R13x77
            | Self::R13x99
            | Self::R13x139 => 13,
            Self::R15x43 | Self::R15x59 | Self::R15x77 | Self::R15x99 | Self::R15x139 => 15,
            Self::R17x43 | Self::R17x59 | Self::R17x77 | Self::R17x99 | Self::R17x139 => 17,
        }
    }
}

/// The version of an encoded symbol.
///
/// The available variants depend on the enabled symbol family features.
#[cfg(any(feature = "qr", feature = "micro-qr", feature = "rmqr"))]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum SymbolVersion {
    /// A Model 2 QR Code version.
    #[cfg(feature = "qr")]
    Qr(QrVersion),
    /// A Micro QR Code version.
    #[cfg(feature = "micro-qr")]
    Micro(MicroVersion),
    /// A Rectangular Micro QR Code version.
    #[cfg(feature = "rmqr")]
    Rmqr(RmqrVersion),
}

/// A validated Model 2 mask number.
#[cfg(feature = "qr")]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct QrMask(u8);

#[cfg(feature = "qr")]
impl QrMask {
    /// Creates a Model 2 mask from a value in `0..=7`.
    #[inline]
    pub const fn new(value: u8) -> Result<Self, EncodeError> {
        if value < 8 { Ok(Self(value)) } else { Err(EncodeError::InvalidMask) }
    }

    /// Returns the mask value.
    #[inline]
    pub const fn value(self) -> u8 {
        self.0
    }
}

/// A validated Micro QR Code mask number.
#[cfg(feature = "micro-qr")]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct MicroMask(u8);

#[cfg(feature = "micro-qr")]
impl MicroMask {
    /// Creates a Micro QR Code mask from a value in `0..=3`.
    #[inline]
    pub const fn new(value: u8) -> Result<Self, EncodeError> {
        if value < 4 { Ok(Self(value)) } else { Err(EncodeError::InvalidMask) }
    }

    /// Returns the mask value.
    #[inline]
    pub const fn value(self) -> u8 {
        self.0
    }
}

/// An Extended Channel Interpretation assignment number.
#[cfg(any(feature = "qr", feature = "rmqr"))]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct EciAssignment(u32);

#[cfg(any(feature = "qr", feature = "rmqr"))]
impl EciAssignment {
    /// The default ISO-8859-1 character set assignment.
    pub const ISO_8859_1: Self = Self(3);
    /// The Shift JIS character set assignment.
    pub const SHIFT_JIS: Self = Self(20);
    /// The UTF-8 character set assignment.
    pub const UTF_8: Self = Self(26);

    /// Creates an ECI assignment from a value in `0..=999999`.
    #[inline]
    pub const fn new(value: u32) -> Result<Self, EncodeError> {
        if value <= 999_999 {
            Ok(Self(value))
        } else {
            Err(EncodeError::InvalidEciAssignment(value))
        }
    }

    /// Returns the ECI assignment number.
    #[inline]
    pub const fn value(self) -> u32 {
        self.0
    }
}

/// A validated FNC1 second-position application indicator.
#[cfg(any(feature = "qr", feature = "rmqr"))]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ApplicationIndicator(u8);

#[cfg(any(feature = "qr", feature = "rmqr"))]
impl ApplicationIndicator {
    /// Creates a numeric application indicator from a value in `0..=99`.
    #[inline]
    pub const fn numeric(value: u8) -> Result<Self, EncodeError> {
        if value <= 99 { Ok(Self(value)) } else { Err(EncodeError::InvalidApplicationIndicator) }
    }

    /// Creates an application indicator from one ASCII letter.
    #[inline]
    pub const fn letter(value: char) -> Result<Self, EncodeError> {
        if value.is_ascii_alphabetic() {
            Ok(Self(value as u8 + 100))
        } else {
            Err(EncodeError::InvalidApplicationIndicator)
        }
    }

    /// Returns the encoded application indicator byte.
    #[inline]
    pub const fn value(self) -> u8 {
        self.0
    }
}

/// The FNC1 header applied to a Model 2 or rMQR symbol.
#[cfg(any(feature = "qr", feature = "rmqr"))]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Fnc1 {
    /// Marks the symbol as GS1 data using FNC1 in first position.
    Gs1,
    /// Marks the symbol as industry data using FNC1 in second position.
    Industry(ApplicationIndicator),
}

/// Metadata carried by a Structured Append symbol.
#[cfg(feature = "qr")]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct StructuredAppendInfo {
    index:  u8,
    total:  u8,
    parity: u8,
}

#[cfg(feature = "qr")]
impl StructuredAppendInfo {
    /// Returns the zero-based symbol index.
    #[inline]
    pub const fn index(self) -> u8 {
        self.index
    }

    /// Returns the total number of symbols.
    #[inline]
    pub const fn total(self) -> u8 {
        self.total
    }

    /// Returns the parity byte shared by the symbol sequence.
    #[inline]
    pub const fn parity(self) -> u8 {
        self.parity
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Mode {
    Numeric,
    Alphanumeric,
    Byte,
    #[cfg_attr(not(feature = "kanji"), allow(dead_code))]
    Kanji,
    #[cfg_attr(not(any(feature = "qr", feature = "rmqr")), allow(dead_code))]
    Eci,
}

impl Mode {
    #[cfg(feature = "qr")]
    #[inline]
    const fn qr_bits(self) -> u8 {
        match self {
            Self::Numeric => 0b0001,
            Self::Alphanumeric => 0b0010,
            Self::Byte => 0b0100,
            Self::Kanji => 0b1000,
            Self::Eci => 0b0111,
        }
    }

    #[cfg(feature = "qr")]
    #[inline]
    const fn cci_bits(self, version: QrVersion) -> u8 {
        let group = version_group(version);

        match self {
            Self::Numeric => [10, 12, 14][group],
            Self::Alphanumeric => [9, 11, 13][group],
            Self::Byte => [8, 16, 16][group],
            Self::Kanji => [8, 10, 12][group],
            Self::Eci => 0,
        }
    }
}

/// An explicitly constructed QR Code data or control segment.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Segment {
    mode:            Mode,
    character_count: usize,
    bits:            BitBuffer,
    source:          Vec<u8>,
}

impl Segment {
    /// Creates a Numeric segment from ASCII digits.
    pub fn numeric(data: impl AsRef<str>) -> Result<Self, EncodeError> {
        let data = data.as_ref();
        let mut bits = BitBuffer::with_capacity(data.len() * 10 / 3 + 4);
        let bytes = data.as_bytes();

        for (offset, byte) in bytes.iter().enumerate() {
            if !byte.is_ascii_digit() {
                return Err(EncodeError::InvalidData {
                    mode:        "numeric",
                    byte_offset: offset,
                });
            }
        }

        for chunk in bytes.chunks(3) {
            let value = chunk.iter().fold(0, |value, byte| value * 10 + u32::from(byte - b'0'));
            bits.append(value, [0, 4, 7, 10][chunk.len()]);
        }

        Ok(Self {
            mode: Mode::Numeric,
            character_count: bytes.len(),
            bits,
            source: bytes.to_vec(),
        })
    }

    /// Creates an Alphanumeric segment from the standard 45-character set.
    pub fn alphanumeric(data: impl AsRef<str>) -> Result<Self, EncodeError> {
        let data = data.as_ref();
        let mut values = Vec::with_capacity(data.len());

        for (offset, byte) in data.bytes().enumerate() {
            let Some(value) = alphanumeric_value(byte) else {
                return Err(EncodeError::InvalidData {
                    mode:        "alphanumeric",
                    byte_offset: offset,
                });
            };
            values.push(value);
        }

        let mut bits = BitBuffer::with_capacity(values.len() * 11 / 2 + 6);

        for chunk in values.chunks(2) {
            if chunk.len() == 2 {
                bits.append(u32::from(chunk[0]) * 45 + u32::from(chunk[1]), 11);
            } else {
                bits.append(u32::from(chunk[0]), 6);
            }
        }

        Ok(Self {
            mode: Mode::Alphanumeric,
            character_count: values.len(),
            bits,
            source: data.as_bytes().to_vec(),
        })
    }

    #[cfg(any(feature = "qr", feature = "rmqr"))]
    fn fnc1_alphanumeric(data: &[u8]) -> Result<Self, EncodeError> {
        let mut values = Vec::with_capacity(data.len());

        for (offset, &byte) in data.iter().enumerate() {
            // A group separator becomes percent and a literal percent is escaped by doubling it.
            if byte == 0x1D {
                values.push(alphanumeric_value(b'%').expect("percent is alphanumeric"));
            } else if byte == b'%' {
                let value = alphanumeric_value(byte).expect("percent is alphanumeric");
                values.extend_from_slice(&[value, value]);
            } else if let Some(value) = alphanumeric_value(byte) {
                values.push(value);
            } else {
                return Err(EncodeError::InvalidData {
                    mode:        "FNC1 alphanumeric",
                    byte_offset: offset,
                });
            }
        }

        let mut bits = BitBuffer::with_capacity(values.len() * 11 / 2 + 6);

        for chunk in values.chunks(2) {
            if chunk.len() == 2 {
                bits.append(u32::from(chunk[0]) * 45 + u32::from(chunk[1]), 11);
            } else {
                bits.append(u32::from(chunk[0]), 6);
            }
        }

        Ok(Self {
            mode: Mode::Alphanumeric,
            character_count: values.len(),
            bits,
            source: data.to_vec(),
        })
    }

    /// Creates a Byte segment without adding an ECI header.
    pub fn bytes(data: impl AsRef<[u8]>) -> Self {
        let data = data.as_ref();

        // A Byte segment stores its payload once in `bits`, so `source` stays empty and is read back from there.
        Self {
            mode:            Mode::Byte,
            character_count: data.len(),
            bits:            BitBuffer::from_bytes(data.to_vec()),
            source:          Vec::new(),
        }
    }

    // Returns the original bytes a segment was built from, reading a Byte segment back from its bit buffer.
    #[cfg(any(feature = "qr", feature = "rmqr"))]
    fn source_bytes(&self) -> &[u8] {
        match self.mode {
            Mode::Byte => self.bits.as_bytes(),
            _ => &self.source,
        }
    }

    /// Creates an ECI control segment.
    #[cfg(any(feature = "qr", feature = "rmqr"))]
    pub fn eci(assignment: EciAssignment) -> Self {
        let mut bits = BitBuffer::with_capacity(24);
        let value = assignment.0;

        // Prefix bits select the shortest one, two or three-codeword ECI representation.
        if value < 128 {
            bits.append(value, 8);
        } else if value < 16_384 {
            bits.append(0b10, 2);
            bits.append(value, 14);
        } else {
            bits.append(0b110, 3);
            bits.append(value, 21);
        }

        Self {
            mode: Mode::Eci,
            character_count: 0,
            bits,
            source: Vec::new(),
        }
    }

    #[cfg(feature = "kanji")]
    /// Creates a Kanji segment from JIS X 0208 characters.
    ///
    /// The eight JIS X 0208 positions whose Unicode mapping is disputed are rejected, as are characters outside JIS X 0208.
    pub fn kanji(data: impl AsRef<str>) -> Result<Self, EncodeError> {
        let data = data.as_ref();
        let mut bits = BitBuffer::with_capacity(data.chars().count() * 13);
        let mut source = Vec::with_capacity(data.len() * 2);
        let mut count = 0;

        for (offset, character) in data.char_indices() {
            let Some((value, encoded)) = kanji_encoding(character) else {
                return Err(EncodeError::InvalidData {
                    mode: "Kanji", byte_offset: offset
                });
            };

            bits.append(u32::from(value), 13);
            source.extend_from_slice(&encoded);
            count += 1;
        }
        Ok(Self {
            mode: Mode::Kanji,
            character_count: count,
            bits,
            source,
        })
    }

    /// Returns the character count stored in this segment.
    #[inline]
    pub const fn character_count(&self) -> usize {
        self.character_count
    }
}

// Reports whether a character is in the ISO/IEC 18004 Table 6 byte set, which leaves 80 to 9F undefined.
#[cfg(any(feature = "qr", feature = "micro-qr", feature = "rmqr"))]
#[inline]
pub(crate) const fn is_latin1_character(character: char) -> bool {
    matches!(character as u32, 0x00..=0x7F | 0xA0..=0xFF)
}

// Reports whether a character keeps its Table 6 byte in a symbol that uses Kanji mode without any ECI header.
// ISO/IEC 18004 warns that readers cannot tell such bytes in E0 to EB from Shift JIS lead bytes, and 80 to 9F are already outside Table 6.
#[cfg(any(feature = "micro-qr", all(feature = "kanji", any(feature = "qr", feature = "rmqr"))))]
#[inline]
pub(crate) const fn is_legacy_byte_character(character: char) -> bool {
    is_latin1_character(character) && !matches!(character as u32, 0xE0..=0xEB)
}

// Rewrites explicit Alphanumeric segments for FNC1, where a literal percent is doubled, and borrows the segments otherwise.
#[cfg(any(feature = "qr", feature = "rmqr"))]
fn normalize_fnc1_segments(
    segments: &[Segment],
    fnc1: bool,
) -> Result<Cow<'_, [Segment]>, EncodeError> {
    if !fnc1 {
        return Ok(Cow::Borrowed(segments));
    }

    segments
        .iter()
        .map(|segment| {
            if segment.mode == Mode::Alphanumeric {
                Segment::fnc1_alphanumeric(segment.source_bytes())
            } else {
                Ok(segment.clone())
            }
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Cow::Owned)
}

// Encodes a character as the JIS8 byte or the JIS X 0208 byte pair that ISO/IEC 18004 defines for ECI 000020.
#[cfg(feature = "kanji")]
fn shift_jis_encoding(character: char) -> Option<([u8; 2], usize)> {
    let mut utf8 = [0; 4];
    let text = character.encode_utf8(&mut utf8);
    let (encoded, _, had_errors) = encoding_rs::SHIFT_JIS.encode(text);

    if had_errors {
        return None;
    }

    match *encoded {
        // JIS8 reads 5C and 7E as the yen sign and the overline, so ASCII backslash and tilde have no JIS8 byte.
        [byte @ (0x5C | 0x7E)] => (!character.is_ascii()).then_some(([byte, 0], 1)),
        // JIS8 reserves 80, which WHATWG uses for U+0080.
        [byte @ (0x00..=0x7F | 0xA1..=0xDF)] => Some(([byte, 0], 1)),
        [lead, trail] if is_unambiguous_jis_x_0208(u16::from_be_bytes([lead, trail])) => {
            Some(([lead, trail], 2))
        },
        _ => None,
    }
}

// Reports whether a Shift JIS byte pair is a JIS X 0208 character with one agreed Unicode mapping.
// NEC row 13 and the IBM extensions are not JIS X 0208 characters, so ISO/IEC 18004 defines no character for them.
// ISO/IEC 18004 gives no Unicode mapping, and the published JIS and Windows tables disagree on the eight listed pairs.
#[cfg(feature = "kanji")]
#[inline]
const fn is_unambiguous_jis_x_0208(value: u16) -> bool {
    matches!(value, 0x8140..=0x84BE | 0x889F..=0x9FFC | 0xE040..=0xEAA4)
        && !matches!(value, 0x815C | 0x815F | 0x8160 | 0x8161 | 0x817C | 0x8191 | 0x8192 | 0x81CA)
}

#[cfg(feature = "kanji")]
fn kanji_encoding(character: char) -> Option<(u16, [u8; 2])> {
    let (bytes, length) = shift_jis_encoding(character)?;

    if length != 2 {
        return None;
    }

    let value = u16::from_be_bytes(bytes);

    // The two valid Shift JIS ranges are shifted into one compact 13-bit index space.
    let adjusted = match value {
        0x8140..=0x9FFC => value - 0x8140,
        0xE040..=0xEBBF => value - 0xC140,
        _ => return None,
    };

    Some(((adjusted >> 8) * 0xC0 + (adjusted & 0xFF), bytes))
}

// Orders the modes used for deterministic tie-breaking in the segment optimizers.
#[cfg(any(feature = "qr", feature = "micro-qr", feature = "rmqr"))]
#[inline]
const fn mode_rank(mode: Mode) -> u8 {
    match mode {
        Mode::Numeric => 0,
        Mode::Alphanumeric => 1,
        Mode::Kanji => 2,
        Mode::Byte => 3,
        Mode::Eci => 4,
    }
}

#[inline]
pub(crate) const fn alphanumeric_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'A'..=b'Z' => Some(byte - b'A' + 10),
        b' ' => Some(36),
        b'$' => Some(37),
        b'%' => Some(38),
        b'*' => Some(39),
        b'+' => Some(40),
        b'-' => Some(41),
        b'.' => Some(42),
        b'/' => Some(43),
        b':' => Some(44),
        _ => None,
    }
}

// Computes the BCH remainder of the data polynomial times x^degree, reduced by the generator.
// The generator is given without its leading term and degree is the number of parity bits.
#[cfg(any(feature = "qr", feature = "micro-qr", feature = "rmqr"))]
#[inline]
pub(crate) const fn bch_remainder(data: u32, generator: u32, degree: u32) -> u32 {
    let mut remainder = data;
    let mut round = 0;

    while round < degree {
        remainder = (remainder << 1) ^ ((remainder >> (degree - 1)) * generator);
        round += 1;
    }

    remainder
}

/// An immutable encoded QR Code, Micro QR Code or rMQR symbol.
#[cfg(any(feature = "qr", feature = "micro-qr", feature = "rmqr"))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Symbol {
    pub(crate) version:           SymbolVersion,
    pub(crate) error_correction:  SymbolErrorCorrection,
    pub(crate) mask:              u8,
    pub(crate) modules:           Vec<bool>,
    #[cfg(feature = "qr")]
    pub(crate) structured_append: Option<StructuredAppendInfo>,
}

#[cfg(any(feature = "qr", feature = "micro-qr", feature = "rmqr"))]
impl Symbol {
    /// Returns the symbol width in modules.
    ///
    /// The square QR Code and Micro QR Code families use this as the height as well, but Rectangular Micro QR Code symbols are not square, so read [`width`](Self::width) and [`height`](Self::height) separately for them.
    #[inline]
    pub const fn size(&self) -> usize {
        self.width()
    }

    /// Returns the symbol width in modules.
    #[inline]
    pub const fn width(&self) -> usize {
        match self.version {
            #[cfg(feature = "qr")]
            SymbolVersion::Qr(version) => version.0 as usize * 4 + 17,
            #[cfg(feature = "micro-qr")]
            SymbolVersion::Micro(version) => version.size(),
            #[cfg(feature = "rmqr")]
            SymbolVersion::Rmqr(version) => version.width(),
        }
    }

    /// Returns the symbol height in modules.
    #[inline]
    pub const fn height(&self) -> usize {
        match self.version {
            #[cfg(feature = "qr")]
            SymbolVersion::Qr(version) => version.0 as usize * 4 + 17,
            #[cfg(feature = "micro-qr")]
            SymbolVersion::Micro(version) => version.size(),
            #[cfg(feature = "rmqr")]
            SymbolVersion::Rmqr(version) => version.height(),
        }
    }

    /// Returns the encoded symbol version.
    #[inline]
    pub const fn version(&self) -> SymbolVersion {
        self.version
    }

    /// Returns the error correction level written to the symbol.
    #[inline]
    pub const fn error_correction(&self) -> SymbolErrorCorrection {
        self.error_correction
    }

    /// Returns the selected mask number.
    #[inline]
    pub const fn mask(&self) -> u8 {
        self.mask
    }

    /// Returns Structured Append metadata when this symbol belongs to a sequence.
    #[cfg(feature = "qr")]
    #[inline]
    pub const fn structured_append(&self) -> Option<StructuredAppendInfo> {
        self.structured_append
    }

    /// Returns a module or `None` when the coordinates are outside the symbol.
    #[inline]
    pub fn module(&self, x: usize, y: usize) -> Option<bool> {
        let width = self.width();
        (x < width && y < self.height()).then(|| self.modules[y * width + x])
    }

    /// Returns every module as one row-major slice without copying.
    ///
    /// A dark module is `true`, and the module at `(x, y)` is at index `y * width + x`.
    #[inline]
    pub fn modules(&self) -> &[bool] {
        &self.modules
    }

    /// Copies the symbol into a row-major Boolean matrix.
    pub fn to_matrix(&self) -> Vec<Vec<bool>> {
        self.modules.chunks(self.width()).map(<[bool]>::to_vec).collect()
    }
}

/// Converts a value to text for QR Code encoding.
///
/// Implementations can use a shorter equivalent spelling when the value has more than one text representation.
/// The optional `url` feature implements this trait for `url::Url`.
pub trait ToQRText {
    /// Returns the text to encode.
    fn to_qr_text(&self) -> String;
}

/// Configures and encodes Model 2 QR Code symbols.
#[cfg(feature = "qr")]
#[derive(Clone, Debug)]
pub struct QrEncoder {
    error_correction:       QrErrorCorrection,
    versions:               RangeInclusive<QrVersion>,
    mask:                   Option<QrMask>,
    boost_error_correction: bool,
    fnc1:                   Option<Fnc1>,
}

#[cfg(feature = "qr")]
impl QrEncoder {
    /// Creates an encoder with automatic mask selection and Model 2 versions 1 through 40.
    #[inline]
    pub const fn new(error_correction: QrErrorCorrection) -> Self {
        Self {
            error_correction,
            versions: QrVersion::MIN..=QrVersion::MAX,
            mask: None,
            boost_error_correction: true,
            fnc1: None,
        }
    }

    /// Selects one exact Model 2 QR Code version.
    #[must_use]
    #[inline]
    pub const fn version(mut self, version: QrVersion) -> Self {
        self.versions = version..=version;
        self
    }

    /// Selects the inclusive Model 2 QR Code version range considered during encoding.
    #[must_use]
    #[inline]
    pub const fn version_range(mut self, versions: RangeInclusive<QrVersion>) -> Self {
        self.versions = versions;
        self
    }

    /// Forces an exact mask instead of selecting one automatically.
    #[must_use]
    #[inline]
    pub const fn mask(mut self, mask: QrMask) -> Self {
        self.mask = Some(mask);
        self
    }

    /// Enables or disables upgrading the error correction level when the selected version has room.
    #[must_use]
    #[inline]
    pub const fn boost_error_correction(mut self, boost: bool) -> Self {
        self.boost_error_correction = boost;
        self
    }

    /// Sets the FNC1 interpretation applied to the complete symbol.
    #[must_use]
    #[inline]
    pub const fn fnc1(mut self, fnc1: Option<Fnc1>) -> Self {
        self.fnc1 = fnc1;
        self
    }

    /// Encodes raw bytes with globally optimized Numeric, Alphanumeric and Byte segments.
    pub fn encode_bytes(&self, data: impl AsRef<[u8]>) -> Result<Symbol, EncodeError> {
        let data = data.as_ref();
        let (capacity, capacity_bits) = self.input_capacity_upper_bound(false)?;

        ensure_input_length(data.len(), capacity, capacity_bits, 1)?;

        self.encode_optimized_bytes(data, None)
    }

    /// Encodes text with globally optimized character modes and ECI transitions.
    pub fn encode_text(&self, text: impl AsRef<str>) -> Result<Symbol, EncodeError> {
        let text = text.as_ref();
        let (capacity, capacity_bits) = self.input_capacity_upper_bound(false)?;

        ensure_input_length(text.chars().count(), capacity, capacity_bits, 1)?;

        self.encode_optimized_text(text, None, optimizer::Start::Free)
    }

    /// Encodes a value after converting it to its QR Code text representation.
    #[inline]
    pub fn encode_to_qr_text<T: ToQRText + ?Sized>(
        &self,
        value: &T,
    ) -> Result<Symbol, EncodeError> {
        let text = value.to_qr_text();
        self.encode_text(&text)
    }

    /// Encodes explicit segments without changing their boundaries.
    ///
    /// The segments are written as given, so keep a Kanji segment after an explicit Shift JIS ECI segment unless the symbol has no ECI segment at all.
    pub fn encode_segments(&self, segments: &[Segment]) -> Result<Symbol, EncodeError> {
        self.encode_segments_with_header(segments, None)
    }

    /// Encodes caller-selected byte parts as one Structured Append sequence.
    pub fn encode_structured_append_bytes(
        &self,
        parts: &[&[u8]],
    ) -> Result<Vec<Symbol>, EncodeError> {
        validate_part_count(parts.len())?;
        let (capacity, capacity_bits) = self.input_capacity_upper_bound(true)?;

        for part in parts {
            ensure_input_length(part.len(), capacity, capacity_bits, 1)?;
        }

        let parity = parts
            .iter()
            .flat_map(|part| part.iter().copied())
            .fold(0, |parity, byte| parity ^ byte);

        parts
            .iter()
            .enumerate()
            .map(|(index, part)| {
                self.encode_optimized_bytes(
                    part,
                    Some(StructuredAppendInfo {
                        index: index as u8,
                        total: parts.len() as u8,
                        parity,
                    }),
                )
            })
            .collect()
    }

    /// Encodes caller-selected text parts as one Structured Append sequence.
    pub fn encode_structured_append_text(
        &self,
        parts: &[&str],
    ) -> Result<Vec<Symbol>, EncodeError> {
        validate_part_count(parts.len())?;
        let (capacity, capacity_bits) = self.input_capacity_upper_bound(true)?;

        for part in parts {
            ensure_input_length(part.chars().count(), capacity, capacity_bits, 1)?;
        }

        let total = parts.len() as u8;
        let mut selected: Option<SequencePlan> = None;

        // Ties keep the layout without ECI headers, which is listed first.
        for regime in SequenceRegime::ALL {
            if let Some(candidate) = self.structured_text_plans(parts, regime)?
                && selected.as_ref().is_none_or(|selected| {
                    (candidate.version, candidate.area) < (selected.version, selected.area)
                })
            {
                selected = Some(candidate);
            }
        }

        let Some(SequencePlan {
            parts: plans, ..
        }) = selected
        else {
            return Err(self.structured_text_error(parts));
        };

        // Parity uses the byte representation selected by the optimizer, including Shift JIS or UTF-8 bytes.
        let parity = plans
            .iter()
            .flat_map(|(_, segments)| segments)
            .flat_map(|segment| segment.source_bytes().iter().copied())
            .fold(0, |parity, byte| parity ^ byte);

        plans
            .into_iter()
            .enumerate()
            .map(|(index, (version, segments))| {
                model2::encode(
                    &segments,
                    version,
                    self.error_correction,
                    self.mask,
                    self.boost_error_correction,
                    self.fnc1,
                    Some(StructuredAppendInfo {
                        index: index as u8,
                        total,
                        parity,
                    }),
                )
            })
            .collect()
    }

    // Picks the smallest version that fits one Structured Append part and returns its optimized segments, or `None` when no version fits.
    fn qr_structured_plan(
        &self,
        text: &str,
        start: optimizer::Start,
    ) -> Result<Option<(QrVersion, Vec<Segment>)>, EncodeError> {
        if self.versions.start() > self.versions.end() {
            return Err(EncodeError::InvalidVersionRange);
        }

        // Only the presence of the header matters for the fit check, so the metadata values are placeholders.
        let header = StructuredAppendInfo {
            index: 0, total: 16, parity: 0
        };
        let range = self.versions.clone();
        let tables = optimizer::TextTables::new(text, self.fnc1.is_some());
        let mut cache = GroupCache::new(|version| {
            optimizer::text(&tables, optimizer::Profile::qr(version), start)
        });

        for value in range.start().0..=range.end().0 {
            let version = QrVersion(value);

            // A start without ECI headers has no plan for characters that need one.
            let Ok(segments) = cache.get(version) else {
                return Ok(None);
            };

            if model2::fits(segments, version, self.error_correction, self.fnc1, Some(header)) {
                return Ok(Some((version, segments.to_vec())));
            }
        }

        Ok(None)
    }

    // Plans every caller-selected part in one ECI layout, minimizing the largest version and then the total area.
    // A part that leaves an ECI in force makes every later part declare one again.
    fn structured_text_plans(
        &self,
        parts: &[&str],
        regime: SequenceRegime,
    ) -> Result<Option<SequencePlan>, EncodeError> {
        if !regime.suits(&parts.concat()) {
            return Ok(None);
        }

        // Candidate plans of each part, indexed by whether an ECI is in force before it.
        let mut options = Vec::with_capacity(parts.len());

        for (index, part) in parts.iter().enumerate() {
            let mut by_state: [Vec<(QrVersion, Vec<Segment>, bool)>; 2] = [Vec::new(), Vec::new()];

            for eci_in_force in [false, true] {
                // The first part never follows an ECI, and the layout without ECI headers never has one in force.
                if eci_in_force && (index == 0 || regime != SequenceRegime::Standard) {
                    continue;
                }

                for &(start, _) in regime.starts(eci_in_force) {
                    if let Some((version, segments)) = self.qr_structured_plan(part, start)? {
                        let leaves_eci = eci_in_force
                            || segments.iter().any(|segment| segment.mode == Mode::Eci);

                        by_state[usize::from(eci_in_force)].push((version, segments, leaves_eci));
                    }
                }
            }

            options.push(by_state);
        }

        let mut caps: Vec<QrVersion> = options
            .iter()
            .flat_map(|by_state| by_state.iter().flatten().map(|(version, ..)| *version))
            .collect();

        caps.sort_unstable();
        caps.dedup();

        for cap in caps {
            // Each entry holds the smallest area to reach the part boundary with or without an ECI in force, and its back pointer.
            let mut best: Vec<[Option<(usize, usize, usize)>; 2]> =
                vec![[None; 2]; parts.len() + 1];

            best[0][0] = Some((0, 0, 0));

            for (index, by_state) in options.iter().enumerate() {
                for eci_in_force in [false, true] {
                    let Some((area, ..)) = best[index][usize::from(eci_in_force)] else {
                        continue;
                    };

                    for (choice, (version, _, leaves_eci)) in
                        by_state[usize::from(eci_in_force)].iter().enumerate()
                    {
                        if *version > cap {
                            continue;
                        }

                        let size = usize::from(version.value()) * 4 + 17;
                        let candidate = (area + size * size, usize::from(eci_in_force), choice);
                        let slot = &mut best[index + 1][usize::from(*leaves_eci)];

                        if slot.is_none_or(|current| candidate.0 < current.0) {
                            *slot = Some(candidate);
                        }
                    }
                }
            }

            let Some((mut state, area)) = (0..2)
                .filter_map(|state| best[parts.len()][state].map(|(area, ..)| (state, area)))
                .min_by_key(|&(_, area)| area)
            else {
                continue;
            };

            let mut plans = Vec::with_capacity(parts.len());

            for index in (0..parts.len()).rev() {
                let (_, previous, choice) = best[index + 1][state].expect("the path is reachable");
                let (version, segments, _) = options[index][previous][choice].clone();

                plans.push((version, segments));
                state = previous;
            }

            plans.reverse();

            return Ok(Some(SequencePlan {
                version: cap,
                area,
                parts: plans,
            }));
        }

        Ok(None)
    }

    // Reproduces the capacity error of the first part that no version can hold.
    fn structured_text_error(&self, parts: &[&str]) -> EncodeError {
        let version = *self.versions.end();
        let capacity_error = EncodeError::DataTooLong {
            required_bits: None,
            capacity_bits: model2::data_codewords(version, self.error_correction) * 8,
        };

        // Later parts may have to restate an ECI, so they are measured with an explicit one.
        for (index, part) in parts.iter().enumerate() {
            let start =
                if index == 0 { optimizer::Start::Default } else { optimizer::Start::Explicit };
            let tables = optimizer::TextTables::new(part, self.fnc1.is_some());
            let Ok(segments) = optimizer::text(&tables, optimizer::Profile::qr(version), start)
            else {
                continue;
            };

            if let Err(error) = model2::encode(
                &segments,
                version,
                self.error_correction,
                self.mask,
                false,
                self.fnc1,
                Some(StructuredAppendInfo {
                    index:  index as u8,
                    total:  parts.len() as u8,
                    parity: 0,
                }),
            ) {
                return error;
            }
        }

        capacity_error
    }

    /// Encodes caller-selected segment parts as one Structured Append sequence.
    ///
    /// Each part must include its own ECI headers before the data that needs them; ECI state is not copied from earlier parts.
    /// A Kanji segment without a Shift JIS ECI segment before it reads as Shift JIS only when no part of the sequence has an ECI segment.
    ///
    /// ```rust
    /// use qrcode_generator::{Segment, qr::{EciAssignment, Encoder, ErrorCorrection}};
    ///
    /// let first = [Segment::eci(EciAssignment::UTF_8), Segment::bytes("café".as_bytes())];
    /// let second = [Segment::eci(EciAssignment::UTF_8), Segment::bytes("世界".as_bytes())];
    /// let symbols = Encoder::new(ErrorCorrection::Medium)
    ///     .encode_structured_append_segments(&[&first, &second])
    ///     .unwrap();
    /// assert_eq!(2, symbols.len());
    /// ```
    pub fn encode_structured_append_segments(
        &self,
        parts: &[&[Segment]],
    ) -> Result<Vec<Symbol>, EncodeError> {
        validate_part_count(parts.len())?;

        let parity = parts
            .iter()
            .flat_map(|part| part.iter())
            .flat_map(|segment| segment.source_bytes().iter().copied())
            .fold(0, |parity, byte| parity ^ byte);

        parts
            .iter()
            .enumerate()
            .map(|(index, part)| {
                self.encode_segments_with_header(
                    part,
                    Some(StructuredAppendInfo {
                        index: index as u8,
                        total: parts.len() as u8,
                        parity,
                    }),
                )
            })
            .collect()
    }

    /// Encodes bytes as one symbol or automatically splits them into at most 16 Structured Append symbols.
    ///
    /// When the data fits one symbol, that lone symbol is returned without Structured Append metadata, because a single symbol needs no sequence header.
    pub fn encode_bytes_with_structured_append(
        &self,
        data: impl AsRef<[u8]>,
    ) -> Result<Vec<Symbol>, EncodeError> {
        let data = data.as_ref();

        match self.encode_bytes(data) {
            Ok(symbol) => return Ok(vec![symbol]),
            Err(EncodeError::DataTooLong {
                ..
            }) => {},
            Err(error) => return Err(error),
        }

        let range = self.structured_append_range()?;
        let (capacity, capacity_bits) = self.input_capacity_upper_bound(true)?;

        ensure_input_length(data.len(), capacity, capacity_bits, 16)?;

        // Partitioning failures replace internal placeholder errors with the full sequence capacity.
        let sequence_capacity_error = || EncodeError::DataTooLong {
            required_bits: None,
            capacity_bits: model2::data_codewords(*range.end(), self.error_correction) * 8 * 16,
        };

        let reach = self.qr_character_upper_bound(*range.end());
        let mut costs = PrefixCosts::new(|start: usize, version, ()| {
            let end = data.len().min(start.saturating_add(reach));

            optimizer::byte_costs(
                &data[start..end],
                optimizer::Profile::qr(version),
                self.fnc1.is_some(),
            )
        });

        // Greedy maximum-size parts determine the minimum possible symbol count.
        let minimum_parts = self
            .minimum_byte_parts(&mut costs, data.len(), range.clone())
            .ok_or_else(sequence_capacity_error)?;

        let candidate = smallest_version_cap(&range, |candidate| {
            self.minimum_byte_parts(&mut costs, data.len(), *range.start()..=candidate)
                == Some(minimum_parts)
        })
        .ok_or_else(sequence_capacity_error)?;
        let parts = self
            .minimum_area_byte_partition(
                &mut costs,
                data.len(),
                minimum_parts,
                *range.start()..=candidate,
            )
            .map_err(|_| sequence_capacity_error())?;

        let slices: Vec<&[u8]> = parts.into_iter().map(|(start, end)| &data[start..end]).collect();

        self.encode_structured_append_bytes(&slices)
    }

    /// Encodes text as one symbol or automatically splits it into at most 16 Structured Append symbols.
    ///
    /// When the text fits one symbol, that lone symbol is returned without Structured Append metadata, because a single symbol needs no sequence header.
    pub fn encode_text_with_structured_append(
        &self,
        text: impl AsRef<str>,
    ) -> Result<Vec<Symbol>, EncodeError> {
        let text = text.as_ref();

        match self.encode_text(text) {
            Ok(symbol) => return Ok(vec![symbol]),
            Err(EncodeError::DataTooLong {
                ..
            }) => {},
            Err(error) => return Err(error),
        }

        let range = self.structured_append_range()?;
        let (capacity, capacity_bits) = self.input_capacity_upper_bound(true)?;

        ensure_input_length(text.chars().count(), capacity, capacity_bits, 16)?;

        let input = SplitText::new(text);
        let reach = self.qr_character_upper_bound(*range.end());
        let mut costs = PrefixCosts::new(|start: usize, version, mode| {
            let end = input.len().min(start.saturating_add(reach));

            optimizer::text_costs(
                input.slice(start, end),
                optimizer::Profile::qr(version),
                self.fnc1.is_some(),
                mode,
            )
        });

        // Partitioning failures replace internal placeholder errors with the full sequence capacity.
        let sequence_capacity_error = || EncodeError::DataTooLong {
            required_bits: None,
            capacity_bits: model2::data_codewords(*range.end(), self.error_correction) * 8 * 16,
        };

        // Each ECI layout is searched on its own, because one ECI anywhere changes how Kanji mode reads in every part.
        // The sequence then minimizes the symbol count, the largest version and the total area, and ties keep the layout without ECI headers.
        let mut selected: Option<SequencePartition> = None;

        for regime in SequenceRegime::ALL {
            if !regime.suits(text) {
                continue;
            }

            // Text partitions use scalar boundaries so no UTF-8 character is split between symbols.
            let Some(minimum_parts) =
                self.minimum_text_parts(&input, &mut costs, range.clone(), regime)
            else {
                continue;
            };

            if selected.as_ref().is_some_and(|selected| minimum_parts > selected.count) {
                continue;
            }

            let Some(candidate) = smallest_version_cap(&range, |candidate| {
                self.minimum_text_parts(&input, &mut costs, *range.start()..=candidate, regime)
                    == Some(minimum_parts)
            }) else {
                continue;
            };

            let (area, parts) = self
                .minimum_area_text_partition(
                    &input,
                    &mut costs,
                    minimum_parts,
                    *range.start()..=candidate,
                    regime,
                )
                .map_err(|_| sequence_capacity_error())?;

            if selected.as_ref().is_none_or(|selected| {
                (minimum_parts, candidate, area) < (selected.count, selected.version, selected.area)
            }) {
                selected = Some(SequencePartition {
                    count: minimum_parts,
                    version: candidate,
                    area,
                    parts,
                });
            }
        }

        let parts = selected.ok_or_else(sequence_capacity_error)?.parts;

        let slices: Vec<&str> =
            parts.into_iter().map(|(start, end)| input.slice(start, end)).collect();
        self.encode_structured_append_text(&slices)
    }

    fn structured_append_range(&self) -> Result<RangeInclusive<QrVersion>, EncodeError> {
        if self.versions.start() > self.versions.end() {
            Err(EncodeError::InvalidVersionRange)
        } else {
            Ok(self.versions.clone())
        }
    }

    fn input_capacity_upper_bound(
        &self,
        structured_append: bool,
    ) -> Result<(usize, usize), EncodeError> {
        if self.versions.start() > self.versions.end() {
            return Err(EncodeError::InvalidVersionRange);
        }

        let version = *self.versions.end();
        let capacity_bits = model2::data_codewords(version, self.error_correction) * 8;
        let overhead_bits = 4
            + usize::from(Mode::Numeric.cci_bits(version))
            + usize::from(structured_append) * 20
            + match self.fnc1 {
                Some(Fnc1::Gs1) => 4,
                Some(Fnc1::Industry(_)) => 12,
                None => 0,
            };

        Ok((numeric_character_capacity(capacity_bits, overhead_bits), capacity_bits))
    }

    // Counts the fewest byte parts by taking the longest part that fits each time.
    fn minimum_byte_parts<F>(
        &self,
        costs: &mut PrefixCosts<(), F>,
        length: usize,
        range: RangeInclusive<QrVersion>,
    ) -> Option<usize>
    where
        F: FnMut(usize, QrVersion, ()) -> Vec<Option<usize>>, {
        let mut start = 0;
        let mut count = 0;

        while start < length {
            if count == 16 {
                return None;
            }

            start = self.farthest_part_end(costs, start, (), range.clone())?;
            count += 1;
        }

        Some(count)
    }

    // Finds the farthest end of a Structured Append part from `start` that fits some version in the range.
    fn farthest_part_end<K, F>(
        &self,
        costs: &mut PrefixCosts<K, F>,
        start: usize,
        key: K,
        range: RangeInclusive<QrVersion>,
    ) -> Option<usize>
    where
        K: Copy + Ord,
        F: FnMut(usize, QrVersion, K) -> Vec<Option<usize>>, {
        let header_bits = model2::header_bits(self.fnc1, true);
        let mut result = None;

        for value in range.start().value()..=range.end().value() {
            let version = QrVersion(value);
            let capacity_bits = model2::data_codewords(version, self.error_correction) * 8;

            // Prefix costs never decrease, so the prefixes that fit all come before the ones that do not.
            let count = costs.get(start, version, key)[1..].partition_point(|cost| {
                cost.is_some_and(|bits| header_bits + bits <= capacity_bits)
            });

            if count > 0 {
                result = result.max(Some(start + count));
            }
        }

        result
    }

    // Counts the fewest parts of one ECI layout, keeping per layer the farthest end with and without an ECI in force.
    fn minimum_text_parts<F>(
        &self,
        input: &SplitText<'_>,
        costs: &mut PrefixCosts<optimizer::Start, F>,
        range: RangeInclusive<QrVersion>,
        regime: SequenceRegime,
    ) -> Option<usize>
    where
        F: FnMut(usize, QrVersion, optimizer::Start) -> Vec<Option<usize>>, {
        let length = input.len();
        let mut layer = vec![(0, false)];

        for count in 1..=16 {
            let mut farthest = [None; 2];

            for &(start, eci_in_force) in &layer {
                for (end, leaves_eci) in self
                    .text_part_ends(input, costs, start, eci_in_force, regime, range.clone())
                    .into_iter()
                    .flatten()
                {
                    if end == length {
                        return Some(count);
                    }

                    let slot = &mut farthest[usize::from(leaves_eci)];

                    if slot.is_none_or(|current| end > current) {
                        *slot = Some(end);
                    }
                }
            }

            // A start without an ECI in force dominates one with it at the same or a nearer end.
            layer.clear();
            layer.extend(farthest[0].map(|end| (end, false)));

            if let Some(end) = farthest[1]
                && farthest[0].is_none_or(|free| end > free)
            {
                layer.push((end, true));
            }

            if layer.is_empty() {
                return None;
            }
        }

        None
    }

    // Finds the farthest ends one text part can reach from a start, each with whether it may leave an ECI in force.
    // A start that may leave an ECI in force is only listed when it reaches farther than the starts that cannot.
    fn text_part_ends<F>(
        &self,
        input: &SplitText<'_>,
        costs: &mut PrefixCosts<optimizer::Start, F>,
        start: usize,
        eci_in_force: bool,
        regime: SequenceRegime,
        range: RangeInclusive<QrVersion>,
    ) -> [Option<(usize, bool)>; 2]
    where
        F: FnMut(usize, QrVersion, optimizer::Start) -> Vec<Option<usize>>, {
        // Numeric mode gives the largest possible scalar capacity for any text input.
        let high =
            input.len().min(start.saturating_add(self.qr_character_upper_bound(*range.end())));
        let mut result = [None; 2];
        let mut farthest = None;

        for (index, &(mode, leaves_eci)) in regime.starts(eci_in_force).iter().enumerate() {
            // Without a character in reach that an ECI could help, a plan allowed to declare one reaches no farther.
            if leaves_eci && !eci_in_force && input.eci_useful_from[start] >= high {
                continue;
            }

            let end = self.farthest_part_end(costs, start, mode, range.clone());

            if let Some(end) = end
                && farthest.is_none_or(|farthest| end > farthest)
            {
                result[index] = Some((end, leaves_eci));
                farthest = Some(end);
            }
        }

        result
    }

    fn minimum_area_byte_partition<F>(
        &self,
        costs: &mut PrefixCosts<(), F>,
        length: usize,
        part_count: usize,
        range: RangeInclusive<QrVersion>,
    ) -> Result<Vec<(usize, usize)>, EncodeError>
    where
        F: FnMut(usize, QrVersion, ()) -> Vec<Option<usize>>, {
        // Byte data never declares an ECI, so every part ends without one in force.
        let (_, parts) = minimum_area_partition(length, part_count, range, |start, _, version| {
            let end = self.farthest_part_end(costs, start, (), version..=version);

            [end.map(|end| (end, false)), None]
        })?;

        Ok(parts)
    }

    fn minimum_area_text_partition<F>(
        &self,
        input: &SplitText<'_>,
        costs: &mut PrefixCosts<optimizer::Start, F>,
        part_count: usize,
        range: RangeInclusive<QrVersion>,
        regime: SequenceRegime,
    ) -> Result<(usize, Vec<(usize, usize)>), EncodeError>
    where
        F: FnMut(usize, QrVersion, optimizer::Start) -> Vec<Option<usize>>, {
        minimum_area_partition(input.len(), part_count, range, |start, eci_in_force, version| {
            self.text_part_ends(input, costs, start, eci_in_force, regime, version..=version)
        })
    }

    #[inline]
    const fn qr_character_upper_bound(&self, version: QrVersion) -> usize {
        model2::data_codewords(version, self.error_correction) * 8 * 3 / 10 + 2
    }

    fn encode_optimized_bytes(
        &self,
        data: &[u8],
        structured_append: Option<StructuredAppendInfo>,
    ) -> Result<Symbol, EncodeError> {
        self.encode_qr_range(
            self.versions.clone(),
            |version| optimizer::bytes(data, optimizer::Profile::qr(version), self.fnc1.is_some()),
            structured_append,
        )
    }

    fn encode_optimized_text(
        &self,
        text: &str,
        structured_append: Option<StructuredAppendInfo>,
        start: optimizer::Start,
    ) -> Result<Symbol, EncodeError> {
        let tables = optimizer::TextTables::new(text, self.fnc1.is_some());

        self.encode_qr_range(
            self.versions.clone(),
            |version| optimizer::text(&tables, optimizer::Profile::qr(version), start),
            structured_append,
        )
    }

    fn encode_qr_range<F>(
        &self,
        range: RangeInclusive<QrVersion>,
        segments: F,
        structured_append: Option<StructuredAppendInfo>,
    ) -> Result<Symbol, EncodeError>
    where
        F: FnMut(QrVersion) -> Result<Vec<Segment>, EncodeError>, {
        if range.start() > range.end() {
            return Err(EncodeError::InvalidVersionRange);
        }

        let mut cache = GroupCache::new(segments);
        let mut last_error = None;

        for value in range.start().0..=range.end().0 {
            let version = QrVersion(value);
            let candidate = cache.get(version)?;

            match model2::encode(
                candidate,
                version,
                self.error_correction,
                self.mask,
                self.boost_error_correction,
                self.fnc1,
                structured_append,
            ) {
                Ok(symbol) => return Ok(symbol),
                Err(
                    error @ EncodeError::DataTooLong {
                        ..
                    },
                ) => last_error = Some(error),
                Err(error) => return Err(error),
            }
        }

        // The version range is verified to be non-empty, so at least one capacity error was stored.
        Err(last_error.unwrap_or(EncodeError::InvalidVersionRange))
    }

    fn encode_segments_with_header(
        &self,
        segments: &[Segment],
        structured_append: Option<StructuredAppendInfo>,
    ) -> Result<Symbol, EncodeError> {
        let segments = normalize_fnc1_segments(segments, self.fnc1.is_some())?;

        self.encode_qr_range(self.versions.clone(), |_| Ok(segments.to_vec()), structured_append)
    }
}

/// Configures and encodes Rectangular Micro QR Code symbols.
#[cfg(feature = "rmqr")]
#[derive(Clone, Debug)]
pub struct RmqrEncoder {
    error_correction:       RmqrErrorCorrection,
    versions:               RangeInclusive<RmqrVersion>,
    boost_error_correction: bool,
    fnc1:                   Option<Fnc1>,
}

#[cfg(feature = "rmqr")]
impl RmqrEncoder {
    /// Creates an encoder that considers every rMQR version.
    #[inline]
    pub const fn new(error_correction: RmqrErrorCorrection) -> Self {
        Self {
            error_correction,
            versions: RmqrVersion::MIN..=RmqrVersion::MAX,
            boost_error_correction: true,
            fnc1: None,
        }
    }

    /// Selects one exact rMQR version.
    #[must_use]
    #[inline]
    pub const fn version(mut self, version: RmqrVersion) -> Self {
        self.versions = version..=version;
        self
    }

    /// Selects an inclusive range in rMQR format indicator order.
    #[must_use]
    #[inline]
    pub const fn version_range(mut self, versions: RangeInclusive<RmqrVersion>) -> Self {
        self.versions = versions;
        self
    }

    /// Enables or disables upgrading Medium error correction to High when the selected version has room.
    #[must_use]
    #[inline]
    pub const fn boost_error_correction(mut self, boost: bool) -> Self {
        self.boost_error_correction = boost;
        self
    }

    /// Sets the FNC1 interpretation applied to the complete symbol.
    #[must_use]
    #[inline]
    pub const fn fnc1(mut self, fnc1: Option<Fnc1>) -> Self {
        self.fnc1 = fnc1;
        self
    }

    /// Encodes raw bytes with globally optimized Numeric, Alphanumeric and Byte segments.
    pub fn encode_bytes(&self, data: impl AsRef<[u8]>) -> Result<Symbol, EncodeError> {
        let data = data.as_ref();
        let (capacity, capacity_bits) = self.input_capacity_upper_bound()?;

        ensure_input_length(data.len(), capacity, capacity_bits, 1)?;
        self.encode_optimized(|version| {
            optimizer::bytes(
                data,
                optimizer::Profile::rmqr(rmqr::cci(version)),
                self.fnc1.is_some(),
            )
        })
    }

    /// Encodes text with globally optimized character modes and ECI transitions.
    pub fn encode_text(&self, text: impl AsRef<str>) -> Result<Symbol, EncodeError> {
        let text = text.as_ref();
        let (capacity, capacity_bits) = self.input_capacity_upper_bound()?;

        ensure_input_length(text.chars().count(), capacity, capacity_bits, 1)?;

        let tables = optimizer::TextTables::new(text, self.fnc1.is_some());

        self.encode_optimized(|version| {
            optimizer::text(
                &tables,
                optimizer::Profile::rmqr(rmqr::cci(version)),
                optimizer::Start::Free,
            )
        })
    }

    /// Encodes a value after converting it to its QR Code text representation.
    #[inline]
    pub fn encode_to_qr_text<T: ToQRText + ?Sized>(
        &self,
        value: &T,
    ) -> Result<Symbol, EncodeError> {
        let text = value.to_qr_text();
        self.encode_text(&text)
    }

    /// Encodes explicit segments without changing their boundaries.
    ///
    /// The segments are written as given, so keep a Kanji segment after an explicit Shift JIS ECI segment unless the symbol has no ECI segment at all.
    pub fn encode_segments(&self, segments: &[Segment]) -> Result<Symbol, EncodeError> {
        let segments = normalize_fnc1_segments(segments, self.fnc1.is_some())?;

        self.encode_candidates(|version| {
            rmqr::encode(
                &segments,
                version,
                self.error_correction,
                self.boost_error_correction,
                self.fnc1,
            )
        })
    }

    fn input_capacity_upper_bound(&self) -> Result<(usize, usize), EncodeError> {
        let versions = self.candidates()?;
        let mut result = None;

        for version in versions {
            let capacity_bits = rmqr::data_codewords(version, self.error_correction) * 8;
            let overhead_bits = 3
                + usize::from(rmqr::cci(version)[0])
                + match self.fnc1 {
                    Some(Fnc1::Gs1) => 3,
                    Some(Fnc1::Industry(_)) => 11,
                    None => 0,
                };
            let capacity = numeric_character_capacity(capacity_bits, overhead_bits);

            if result.is_none_or(|(best, _)| capacity > best) {
                result = Some((capacity, capacity_bits));
            }
        }

        result.ok_or(EncodeError::InvalidVersionRange)
    }

    // Segments are planned once per character count indicator profile, because versions sharing one profile share their plan.
    fn encode_optimized<F>(&self, mut optimize: F) -> Result<Symbol, EncodeError>
    where
        F: FnMut(RmqrVersion) -> Result<Vec<Segment>, EncodeError>, {
        let mut cache: Vec<([u8; 4], Vec<Segment>)> = Vec::with_capacity(15);

        self.encode_candidates(|version| {
            let profile = rmqr::cci(version);
            let index = match cache.iter().position(|(cci, _)| *cci == profile) {
                Some(index) => index,
                None => {
                    cache.push((profile, optimize(version)?));
                    cache.len() - 1
                },
            };

            rmqr::encode(
                &cache[index].1,
                version,
                self.error_correction,
                self.boost_error_correction,
                self.fnc1,
            )
        })
    }

    // Tries every candidate version in area order and keeps the last capacity error for the caller.
    fn encode_candidates<F>(&self, mut encode: F) -> Result<Symbol, EncodeError>
    where
        F: FnMut(RmqrVersion) -> Result<Symbol, EncodeError>, {
        let versions = self.candidates()?;
        let mut last_error = None;

        for version in versions {
            match encode(version) {
                Ok(symbol) => return Ok(symbol),
                Err(
                    error @ EncodeError::DataTooLong {
                        ..
                    },
                ) => last_error = Some(error),
                Err(error) => return Err(error),
            }
        }

        Err(last_error.unwrap_or(EncodeError::InvalidVersionRange))
    }

    fn candidates(&self) -> Result<Vec<RmqrVersion>, EncodeError> {
        if self.versions.start() > self.versions.end() {
            return Err(EncodeError::InvalidVersionRange);
        }

        Ok(RmqrVersion::ALL_BY_AREA
            .into_iter()
            .filter(|version| self.versions.contains(version))
            .collect())
    }
}

/// Configures and encodes Micro QR Code symbols.
#[cfg(feature = "micro-qr")]
#[derive(Clone, Debug)]
pub struct MicroEncoder {
    error_correction:       MicroErrorCorrection,
    versions:               RangeInclusive<MicroVersion>,
    mask:                   Option<MicroMask>,
    boost_error_correction: bool,
}

#[cfg(feature = "micro-qr")]
impl MicroEncoder {
    /// Creates an encoder with automatic mask selection and all Micro QR Code versions.
    #[inline]
    pub const fn new(error_correction: MicroErrorCorrection) -> Self {
        Self {
            error_correction,
            versions: MicroVersion::M1..=MicroVersion::M4,
            mask: None,
            boost_error_correction: true,
        }
    }

    /// Selects one exact Micro QR Code version.
    #[must_use]
    #[inline]
    pub const fn version(mut self, version: MicroVersion) -> Self {
        self.versions = version..=version;
        self
    }

    /// Selects the inclusive Micro QR Code version range considered during encoding.
    #[must_use]
    #[inline]
    pub const fn version_range(mut self, versions: RangeInclusive<MicroVersion>) -> Self {
        self.versions = versions;
        self
    }

    /// Forces an exact mask instead of selecting one automatically.
    #[must_use]
    #[inline]
    pub const fn mask(mut self, mask: MicroMask) -> Self {
        self.mask = Some(mask);
        self
    }

    /// Enables or disables upgrading the error correction level when the selected version has room.
    #[must_use]
    #[inline]
    pub const fn boost_error_correction(mut self, boost: bool) -> Self {
        self.boost_error_correction = boost;
        self
    }

    /// Encodes raw bytes with globally optimized modes supported by each candidate version.
    pub fn encode_bytes(&self, data: impl AsRef<[u8]>) -> Result<Symbol, EncodeError> {
        let data = data.as_ref();
        let (version, capacity, capacity_bits) = self.input_capacity_upper_bound()?;

        micro::check_bytes(data, version, capacity, capacity_bits)?;

        self.encode_range(|version| micro::optimize(data, version))
    }

    /// Encodes text with globally optimized modes supported by each candidate version.
    pub fn encode_text(&self, text: impl AsRef<str>) -> Result<Symbol, EncodeError> {
        let text = text.as_ref();
        let (version, capacity, capacity_bits) = self.input_capacity_upper_bound()?;
        let input = micro::Text::new(text, version, capacity, capacity_bits)?;

        self.encode_range(|version| micro::optimize_text(&input, version))
    }

    /// Encodes a value after converting it to its QR Code text representation.
    #[inline]
    pub fn encode_to_qr_text<T: ToQRText + ?Sized>(
        &self,
        value: &T,
    ) -> Result<Symbol, EncodeError> {
        let text = value.to_qr_text();
        self.encode_text(&text)
    }

    /// Encodes explicit segments without changing their boundaries.
    ///
    /// The segments are written as given, so avoid Byte segments holding E0 to EB in a symbol with Kanji segments, because readers could take them for Shift JIS lead bytes.
    /// An empty Numeric segment is left out, because Micro QR Code writes it with the same bits as the terminator.
    pub fn encode_segments(&self, segments: &[Segment]) -> Result<Symbol, EncodeError> {
        self.encode_range(|_| {
            Ok(segments
                .iter()
                .filter(|segment| segment.mode != Mode::Numeric || segment.character_count > 0)
                .cloned()
                .collect())
        })
    }

    // Returns the candidate version holding the most characters, which is also the most permissive one.
    fn input_capacity_upper_bound(&self) -> Result<(MicroVersion, usize, usize), EncodeError> {
        if self.versions.start() > self.versions.end() {
            return Err(EncodeError::InvalidVersionRange);
        }

        micro_versions(self.versions.clone())
            .filter_map(|version| {
                micro::input_capacity_upper_bound(version, self.error_correction)
                    .map(|(capacity, capacity_bits)| (version, capacity, capacity_bits))
            })
            .max_by_key(|&(_, capacity, _)| capacity)
            .ok_or(EncodeError::UnsupportedErrorCorrection {
                version:          SymbolVersion::Micro(*self.versions.end()),
                error_correction: self.error_correction.into(),
            })
    }

    fn encode_range<F>(&self, mut segments: F) -> Result<Symbol, EncodeError>
    where
        F: FnMut(MicroVersion) -> Result<Vec<Segment>, EncodeError>, {
        if self.versions.start() > self.versions.end() {
            return Err(EncodeError::InvalidVersionRange);
        }

        let mut last_error = None;

        // Unsupported modes and error correction levels eliminate only the current candidate version.
        for version in micro_versions(self.versions.clone()) {
            if micro::input_capacity_upper_bound(version, self.error_correction).is_none() {
                last_error = Some(EncodeError::UnsupportedErrorCorrection {
                    version:          SymbolVersion::Micro(version),
                    error_correction: self.error_correction.into(),
                });
                continue;
            }

            let result = segments(version).and_then(|segments| {
                micro::encode(
                    &segments,
                    version,
                    self.error_correction,
                    self.mask,
                    self.boost_error_correction,
                )
            });

            match result {
                Ok(symbol) => return Ok(symbol),
                Err(error) if candidate_rejection(&error) => last_error = Some(error),
                Err(error) => return Err(error),
            }
        }

        Err(last_error.unwrap_or(EncodeError::InvalidVersionRange))
    }
}

/// Tries a configured Micro QR Code encoder before a configured Model 2 QR Code encoder, unless the Model 2 encoder requires FNC1.
#[cfg(all(feature = "qr", feature = "micro-qr"))]
#[derive(Clone, Debug)]
pub struct AutoEncoder {
    qr:    QrEncoder,
    micro: MicroEncoder,
}

#[cfg(all(feature = "qr", feature = "micro-qr"))]
impl AutoEncoder {
    /// Creates an encoder from independently configured Model 2 and Micro QR Code encoders.
    #[inline]
    pub const fn new(qr: QrEncoder, micro: MicroEncoder) -> Self {
        Self {
            qr,
            micro,
        }
    }

    /// Encodes raw bytes using the smallest eligible symbol family.
    pub fn encode_bytes(&self, data: impl AsRef<[u8]>) -> Result<Symbol, EncodeError> {
        let data = data.as_ref();
        if self.qr.fnc1.is_some() {
            return self.qr.encode_bytes(data);
        }
        match self.micro.encode_bytes(data) {
            Ok(symbol) => Ok(symbol),
            Err(error) if candidate_rejection(&error) => self.qr.encode_bytes(data),
            Err(error) => Err(error),
        }
    }

    /// Encodes text using the smallest eligible symbol family.
    pub fn encode_text(&self, text: impl AsRef<str>) -> Result<Symbol, EncodeError> {
        let text = text.as_ref();
        if self.qr.fnc1.is_some() {
            return self.qr.encode_text(text);
        }
        match self.micro.encode_text(text) {
            Ok(symbol) => Ok(symbol),
            Err(error) if candidate_rejection(&error) => self.qr.encode_text(text),
            Err(error) => Err(error),
        }
    }

    /// Encodes a value after converting it to its QR Code text representation once.
    #[inline]
    pub fn encode_to_qr_text<T: ToQRText + ?Sized>(
        &self,
        value: &T,
    ) -> Result<Symbol, EncodeError> {
        let text = value.to_qr_text();
        self.encode_text(&text)
    }

    /// Encodes explicit segments using the smallest eligible symbol family.
    pub fn encode_segments(&self, segments: &[Segment]) -> Result<Symbol, EncodeError> {
        if self.qr.fnc1.is_some() {
            return self.qr.encode_segments(segments);
        }
        match self.micro.encode_segments(segments) {
            Ok(symbol) => Ok(symbol),
            Err(error) if candidate_rejection(&error) => self.qr.encode_segments(segments),
            Err(error) => Err(error),
        }
    }
}

#[cfg(feature = "qr")]
#[derive(Clone, Copy)]
struct PartitionState {
    end:          usize,
    area:         usize,
    previous:     usize,
    // Whether an ECI is in force at the end, so the next part must declare one again.
    eci_in_force: bool,
}

// Finds the partition into exactly `part_count` parts with the smallest total symbol area, and returns that area with the parts.
// `part_ends` lists the farthest ends one part can reach from a start in one version, each with the ECI state it leaves.
#[cfg(feature = "qr")]
fn minimum_area_partition<F>(
    length: usize,
    part_count: usize,
    range: RangeInclusive<QrVersion>,
    mut part_ends: F,
) -> Result<(usize, Vec<(usize, usize)>), EncodeError>
where
    F: FnMut(usize, bool, QrVersion) -> [Option<(usize, bool)>; 2], {
    // Each layer represents one additional symbol in the fixed-size Structured Append sequence.
    let mut layers = vec![vec![PartitionState {
        end:          0,
        area:         0,
        previous:     0,
        eci_in_force: false,
    }]];

    for _ in 0..part_count {
        let previous_layer = layers.last().expect("the initial layer exists");
        let mut by_end = BTreeMap::new();

        for (previous, state) in previous_layer.iter().enumerate() {
            for value in range.start().value()..=range.end().value() {
                let version = QrVersion(value);
                let size = usize::from(version.value()) * 4 + 17;

                for (end, eci_in_force) in
                    part_ends(state.end, state.eci_in_force, version).into_iter().flatten()
                {
                    let candidate = PartitionState {
                        end,
                        area: state.area + size * size,
                        previous,
                        eci_in_force,
                    };

                    // The key orders states without an ECI in force after the others at the same end, so they come first in reverse.
                    by_end
                        .entry((end, !eci_in_force))
                        .and_modify(|current: &mut PartitionState| {
                            if candidate.area < current.area {
                                *current = candidate;
                            }
                        })
                        .or_insert(candidate);
                }
            }
        }

        let mut best_free_area = usize::MAX;
        let mut best_area = usize::MAX;
        let mut layer = Vec::new();

        // A state that reaches farther with no greater area dominates every earlier state, unless only the earlier one is free of an ECI in force.
        for state in by_end.into_values().rev() {
            if state.eci_in_force {
                if state.area < best_area {
                    best_area = state.area;
                    layer.push(state);
                }
            } else if state.area < best_free_area {
                best_free_area = state.area;
                best_area = best_area.min(state.area);
                layer.push(state);
            }
        }

        layer.reverse();

        if layer.is_empty() {
            return Err(EncodeError::DataTooLong {
                required_bits: None, capacity_bits: 0
            });
        }

        layers.push(layer);
    }

    let (mut state_index, area) = layers[part_count]
        .iter()
        .enumerate()
        .filter(|(_, state)| state.end == length)
        .map(|(index, state)| (index, state.area))
        .min_by_key(|&(_, area)| area)
        .ok_or(EncodeError::DataTooLong {
            required_bits: None, capacity_bits: 0
        })?;

    let mut result = Vec::with_capacity(part_count);

    for layer_index in (1..=part_count).rev() {
        let state = layers[layer_index][state_index];
        let previous = layers[layer_index - 1][state.previous];
        result.push((previous.end, state.end));
        state_index = state.previous;
    }

    result.reverse();

    Ok((area, result))
}

// Finds the smallest largest version that still reaches the wanted symbol count, because raising it never adds symbols.
#[cfg(feature = "qr")]
fn smallest_version_cap<F>(
    range: &RangeInclusive<QrVersion>,
    mut reaches_count: F,
) -> Option<QrVersion>
where
    F: FnMut(QrVersion) -> bool, {
    let mut low = range.start().value();
    let mut high = range.end().value();
    let mut result = None;

    while low <= high {
        let middle = low + (high - low) / 2;

        if reaches_count(QrVersion(middle)) {
            result = Some(QrVersion(middle));
            high = middle - 1;
        } else {
            low = middle + 1;
        }
    }

    result
}

#[cfg(feature = "micro-qr")]
fn micro_versions(versions: RangeInclusive<MicroVersion>) -> impl Iterator<Item = MicroVersion> {
    let start = *versions.start();
    let end = *versions.end();

    [MicroVersion::M1, MicroVersion::M2, MicroVersion::M3, MicroVersion::M4]
        .into_iter()
        .filter(move |version| *version >= start && *version <= end)
}

#[cfg(feature = "micro-qr")]
#[inline]
const fn candidate_rejection(error: &EncodeError) -> bool {
    matches!(
        error,
        EncodeError::DataTooLong { .. }
            | EncodeError::UnsupportedMode { .. }
            | EncodeError::UnsupportedErrorCorrection { .. }
            | EncodeError::TextNotRepresentable { .. }
    )
}

#[cfg(feature = "qr")]
#[inline]
const fn version_group(version: QrVersion) -> usize {
    if version.0 <= 9 {
        0
    } else if version.0 <= 26 {
        1
    } else {
        2
    }
}

// Caches segments per version group during a version scan, because character count indicator widths change only at versions 10 and 27.
#[cfg(feature = "qr")]
struct GroupCache<F> {
    segments: F,
    cache:    [Option<Vec<Segment>>; 3],
}

#[cfg(feature = "qr")]
impl<F: FnMut(QrVersion) -> Result<Vec<Segment>, EncodeError>> GroupCache<F> {
    fn new(segments: F) -> Self {
        Self {
            segments,
            cache: [None, None, None],
        }
    }

    fn get(&mut self, version: QrVersion) -> Result<&[Segment], EncodeError> {
        let group = version_group(version);

        if self.cache[group].is_none() {
            self.cache[group] = Some((self.segments)(version)?);
        }

        Ok(self.cache[group].as_deref().expect("the version group is cached"))
    }
}

#[cfg(feature = "qr")]
#[inline]
const fn validate_part_count(count: usize) -> Result<(), EncodeError> {
    if count >= 1 && count <= 16 {
        Ok(())
    } else {
        Err(EncodeError::InvalidStructuredAppendPartCount {
            count,
        })
    }
}

// Caches the fewest bits of every prefix from one start per version group and key, so partition probes share one optimizer run.
#[cfg(feature = "qr")]
struct PrefixCosts<K, F> {
    compute: F,
    costs:   BTreeMap<(usize, usize, K), Vec<Option<usize>>>,
}

#[cfg(feature = "qr")]
impl<K: Copy + Ord, F: FnMut(usize, QrVersion, K) -> Vec<Option<usize>>> PrefixCosts<K, F> {
    #[inline]
    fn new(compute: F) -> Self {
        Self {
            compute,
            costs: BTreeMap::new(),
        }
    }

    // Returns the prefix costs from `start`, indexed by the prefix length, using the profile of the version group.
    fn get(&mut self, start: usize, version: QrVersion, key: K) -> &[Option<usize>] {
        let compute = &mut self.compute;

        self.costs
            .entry((start, version_group(version), key))
            .or_insert_with(|| compute(start, version, key))
    }
}

// A text prepared for Structured Append splitting at character boundaries.
#[cfg(feature = "qr")]
struct SplitText<'a> {
    text:            &'a str,
    // Byte offsets of every character boundary, ending with the text length.
    offsets:         Vec<usize>,
    // The index of the first character at or after each position that could make an ECI header worthwhile, or the length when none follows.
    eci_useful_from: Vec<usize>,
}

#[cfg(feature = "qr")]
impl<'a> SplitText<'a> {
    fn new(text: &'a str) -> Self {
        let mut offsets: Vec<usize> = text.char_indices().map(|(offset, _)| offset).collect();

        offsets.push(text.len());

        let useful: Vec<bool> = text.chars().map(eci_may_help).collect();
        let mut eci_useful_from = vec![useful.len(); useful.len() + 1];

        for index in (0..useful.len()).rev() {
            eci_useful_from[index] = if useful[index] { index } else { eci_useful_from[index + 1] };
        }

        Self {
            text,
            offsets,
            eci_useful_from,
        }
    }

    // Returns the number of characters.
    #[inline]
    fn len(&self) -> usize {
        self.offsets.len() - 1
    }

    // Returns the characters in `start..end`.
    #[inline]
    fn slice(&self, start: usize, end: usize) -> &'a str {
        &self.text[self.offsets[start]..self.offsets[end]]
    }
}

// Reports whether an ECI header could shorten the plan of a character: it has no Table 6 byte, or Kanji mode under ECI 000020 can hold it.
#[cfg(feature = "qr")]
#[inline]
fn eci_may_help(character: char) -> bool {
    #[cfg(feature = "kanji")]
    if kanji_encoding(character).is_some() {
        return true;
    }

    !is_latin1_character(character)
}

// The parts planned for a Structured Append text sequence, with its largest version and total area.
#[cfg(feature = "qr")]
struct SequencePlan {
    version: QrVersion,
    area:    usize,
    parts:   Vec<(QrVersion, Vec<Segment>)>,
}

// The character ranges chosen for an automatically split Structured Append text sequence, with its symbol count, largest version and total area.
#[cfg(feature = "qr")]
struct SequencePartition {
    count:   usize,
    version: QrVersion,
    area:    usize,
    parts:   Vec<(usize, usize)>,
}

// The ECI layouts a Structured Append text sequence can use, because a reader joins its parts into one data stream.
#[cfg(feature = "qr")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SequenceRegime {
    // No part declares an ECI, so Kanji mode without an ECI header reads as Shift JIS throughout the sequence.
    #[cfg(feature = "kanji")]
    Legacy,
    // Kanji mode follows an explicit Shift JIS ECI, and every part after the first ECI declares its own.
    Standard,
}

#[cfg(feature = "qr")]
impl SequenceRegime {
    #[cfg(feature = "kanji")]
    const ALL: [Self; 2] = [Self::Legacy, Self::Standard];
    #[cfg(not(feature = "kanji"))]
    const ALL: [Self; 1] = [Self::Standard];

    // Lists the start modes of a part, each with whether its plan may leave an ECI in force for the next part.
    #[inline]
    const fn starts(self, eci_in_force: bool) -> &'static [(optimizer::Start, bool)] {
        match self {
            #[cfg(feature = "kanji")]
            Self::Legacy => &[(optimizer::Start::Legacy, false)],
            // A part after an ECI must declare it again right after the Structured Append header.
            Self::Standard if eci_in_force => &[(optimizer::Start::Explicit, true)],
            Self::Standard => {
                &[(optimizer::Start::DefaultEciFree, false), (optimizer::Start::Default, true)]
            },
        }
    }

    // Reports whether this layout can hold every character and may do better than the standard layout.
    #[cfg_attr(not(feature = "kanji"), allow(unused_variables))]
    fn suits(self, text: &str) -> bool {
        match self {
            // Without any Kanji character, the standard layout already covers every plan without ECI headers.
            #[cfg(feature = "kanji")]
            Self::Legacy => {
                text.chars().any(|character| kanji_encoding(character).is_some())
                    && text.chars().all(|character| {
                        is_legacy_byte_character(character) || kanji_encoding(character).is_some()
                    })
            },
            Self::Standard => true,
        }
    }
}
