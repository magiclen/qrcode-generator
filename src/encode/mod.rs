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

        Self {
            mode:            Mode::Byte,
            character_count: data.len(),
            bits:            BitBuffer::from_bytes(data.to_vec()),
            source:          data.to_vec(),
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
    /// Creates a Kanji segment from characters in the supported Shift JIS ranges.
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

#[cfg(feature = "kanji")]
fn kanji_encoding(character: char) -> Option<(u16, [u8; 2])> {
    let mut utf8 = [0; 4];
    let text = character.encode_utf8(&mut utf8);
    let (encoded, _, had_errors) = encoding_rs::SHIFT_JIS.encode(text);

    if had_errors || encoded.len() != 2 {
        return None;
    }

    let bytes = [encoded[0], encoded[1]];
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

        self.encode_optimized_text(text, None, false)
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

        let force_initial_eci = parts.iter().any(|part| requires_non_default_eci(part));
        let total = parts.len() as u8;

        // Each part is optimized once, and both the shared parity and the final symbols reuse the result.
        let mut plans = Vec::with_capacity(parts.len());
        let mut parity = 0u8;

        for part in parts {
            let (version, segments) = self.qr_structured_plan(part, force_initial_eci)?;

            // Parity uses the byte representation selected by the optimizer, including Shift JIS or UTF-8 bytes.
            for segment in &segments {
                for &byte in &segment.source {
                    parity ^= byte;
                }
            }

            plans.push((version, segments));
        }

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

    // Picks the smallest version that fits one Structured Append part and returns its optimized segments.
    fn qr_structured_plan(
        &self,
        text: &str,
        force_initial_eci: bool,
    ) -> Result<(QrVersion, Vec<Segment>), EncodeError> {
        if self.versions.start() > self.versions.end() {
            return Err(EncodeError::InvalidVersionRange);
        }

        // Only the presence of the header matters for the fit check, so the metadata values are placeholders.
        let header = StructuredAppendInfo {
            index: 0, total: 16, parity: 0
        };
        let range = self.versions.clone();
        let mut cache: [Option<Vec<Segment>>; 3] = [None, None, None];

        for value in range.start().0..=range.end().0 {
            let version = QrVersion(value);
            let group = version_group(version);

            if cache[group].is_none() {
                cache[group] = Some(optimizer::text(
                    text,
                    optimizer::Profile::qr(version),
                    self.fnc1.is_some(),
                    force_initial_eci,
                )?);
            }

            let segments = cache[group].as_deref().expect("the version group is cached");

            if model2::fits(segments, version, self.error_correction, self.fnc1, Some(header)) {
                return Ok((version, segments.to_vec()));
            }
        }

        // No version fits, so the largest one is returned to reproduce the same capacity error later.
        let version = *range.end();
        let group = version_group(version);

        if cache[group].is_none() {
            cache[group] = Some(optimizer::text(
                text,
                optimizer::Profile::qr(version),
                self.fnc1.is_some(),
                force_initial_eci,
            )?);
        }

        Ok((version, cache[group].take().expect("the final version group is cached")))
    }

    /// Encodes caller-selected segment parts as one Structured Append sequence.
    pub fn encode_structured_append_segments(
        &self,
        parts: &[&[Segment]],
    ) -> Result<Vec<Symbol>, EncodeError> {
        validate_part_count(parts.len())?;

        let parity = parts
            .iter()
            .flat_map(|part| part.iter())
            .flat_map(|segment| segment.source.iter().copied())
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

        // Greedy maximum-size parts determine the minimum possible symbol count.
        let minimum_parts = self.partition_bytes(data, range.clone())?.len();

        let mut selected = None;

        for value in range.start().value()..=range.end().value() {
            let candidate = QrVersion(value);

            if let Ok(parts) = self.partition_bytes(data, *range.start()..=candidate)
                && parts.len() == minimum_parts
            {
                selected = Some(self.minimum_area_byte_partition(
                    data,
                    minimum_parts,
                    *range.start()..=candidate,
                )?);
                break;
            }
        }

        let parts = selected.ok_or(EncodeError::DataTooLong {
            required_bits: None,
            capacity_bits: model2::data_codewords(*range.end(), self.error_correction) * 8 * 16,
        })?;

        let slices: Vec<&[u8]> = parts.into_iter().map(|(start, end)| &data[start..end]).collect();

        self.encode_structured_append_bytes(&slices)
    }

    /// Encodes text as one symbol or automatically splits it into at most 16 Structured Append symbols.
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

        let offsets = text_boundaries(text);

        // Text partitions use scalar boundaries so no UTF-8 character is split between symbols.
        let minimum_parts = self.partition_text(text, &offsets, range.clone())?.len();

        let mut selected = None;

        for value in range.start().value()..=range.end().value() {
            let candidate = QrVersion(value);

            if let Ok(parts) = self.partition_text(text, &offsets, *range.start()..=candidate)
                && parts.len() == minimum_parts
            {
                selected = Some(self.minimum_area_text_partition(
                    text,
                    &offsets,
                    minimum_parts,
                    *range.start()..=candidate,
                )?);
                break;
            }
        }

        let parts = selected.ok_or(EncodeError::DataTooLong {
            required_bits: None,
            capacity_bits: model2::data_codewords(*range.end(), self.error_correction) * 8 * 16,
        })?;

        let slices: Vec<&str> =
            parts.into_iter().map(|(start, end)| &text[offsets[start]..offsets[end]]).collect();
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

    fn partition_bytes(
        &self,
        data: &[u8],
        range: RangeInclusive<QrVersion>,
    ) -> Result<Vec<(usize, usize)>, EncodeError> {
        let maximum_version = *range.end();
        let mut result = Vec::new();
        let mut start = 0;

        while start < data.len() {
            if result.len() == 16 {
                return Err(EncodeError::DataTooLong {
                    required_bits: None, capacity_bits: 0
                });
            }

            let mut low = start + 1;

            // Numeric mode gives the largest possible character capacity for any input.
            let mut high = data
                .len()
                .min(start.saturating_add(self.qr_character_upper_bound(maximum_version)));

            let mut fitting = None;

            while low <= high {
                let middle = low + (high - low) / 2;

                if self.bytes_fit(&data[start..middle], range.clone()) {
                    fitting = Some(middle);
                    low = middle + 1;
                } else {
                    high = middle - 1;
                }
            }

            let end = fitting
                .ok_or(EncodeError::DataTooLong {
                    required_bits: None, capacity_bits: 0
                })?;

            result.push((start, end));
            start = end;
        }
        Ok(result)
    }

    fn partition_text(
        &self,
        text: &str,
        offsets: &[usize],
        range: RangeInclusive<QrVersion>,
    ) -> Result<Vec<(usize, usize)>, EncodeError> {
        let maximum_version = *range.end();
        let force_initial_eci = requires_non_default_eci(text);

        let mut result = Vec::new();
        let mut start = 0;
        let length = offsets.len() - 1;

        while start < length {
            if result.len() == 16 {
                return Err(EncodeError::DataTooLong {
                    required_bits: None, capacity_bits: 0
                });
            }

            let mut low = start + 1;

            // Numeric mode gives the largest possible scalar capacity for any text input.
            let mut high =
                length.min(start.saturating_add(self.qr_character_upper_bound(maximum_version)));

            let mut fitting = None;

            while low <= high {
                let middle = low + (high - low) / 2;

                if self.text_fits(
                    &text[offsets[start]..offsets[middle]],
                    range.clone(),
                    force_initial_eci,
                ) {
                    fitting = Some(middle);
                    low = middle + 1;
                } else {
                    high = middle - 1;
                }
            }

            let end = fitting
                .ok_or(EncodeError::DataTooLong {
                    required_bits: None, capacity_bits: 0
                })?;

            result.push((start, end));

            start = end;
        }
        Ok(result)
    }

    fn minimum_area_byte_partition(
        &self,
        data: &[u8],
        part_count: usize,
        range: RangeInclusive<QrVersion>,
    ) -> Result<Vec<(usize, usize)>, EncodeError> {
        minimum_area_partition(data.len(), part_count, range, |start, version| {
            let high = data.len().min(start.saturating_add(self.qr_character_upper_bound(version)));

            maximum_fitting_end(high, start, |end| {
                self.bytes_fit(&data[start..end], version..=version)
            })
        })
    }

    fn minimum_area_text_partition(
        &self,
        text: &str,
        offsets: &[usize],
        part_count: usize,
        range: RangeInclusive<QrVersion>,
    ) -> Result<Vec<(usize, usize)>, EncodeError> {
        let length = offsets.len() - 1;
        let force_initial_eci = requires_non_default_eci(text);

        minimum_area_partition(length, part_count, range, |start, version| {
            let high = length.min(start.saturating_add(self.qr_character_upper_bound(version)));

            maximum_fitting_end(high, start, |end| {
                self.text_fits(
                    &text[offsets[start]..offsets[end]],
                    version..=version,
                    force_initial_eci,
                )
            })
        })
    }

    // Reports whether the bytes fit some version in the range as one Structured Append part.
    fn bytes_fit(&self, data: &[u8], range: RangeInclusive<QrVersion>) -> bool {
        self.qr_range_fits(range, |version| {
            optimizer::bytes(data, optimizer::Profile::qr(version), self.fnc1.is_some())
        })
    }

    // Reports whether the text fits some version in the range as one Structured Append part.
    fn text_fits(
        &self,
        text: &str,
        range: RangeInclusive<QrVersion>,
        force_initial_eci: bool,
    ) -> bool {
        self.qr_range_fits(range, |version| {
            optimizer::text(
                text,
                optimizer::Profile::qr(version),
                self.fnc1.is_some(),
                force_initial_eci,
            )
        })
    }

    // Runs the fit check without drawing a symbol, so probing skips the matrix and mask work.
    fn qr_range_fits<F>(&self, range: RangeInclusive<QrVersion>, mut segments: F) -> bool
    where
        F: FnMut(QrVersion) -> Result<Vec<Segment>, EncodeError>, {
        if range.start() > range.end() {
            return false;
        }

        // Only the presence of the header matters for the fit check, so the values are placeholders.
        let header = StructuredAppendInfo {
            index: 0, total: 16, parity: 0
        };
        let mut cache: [Option<Vec<Segment>>; 3] = [None, None, None];

        for value in range.start().0..=range.end().0 {
            let version = QrVersion(value);
            let group = version_group(version);

            if cache[group].is_none() {
                let Ok(candidate) = segments(version) else {
                    return false;
                };

                cache[group] = Some(candidate);
            }

            let candidate = cache[group].as_deref().expect("the version group is cached");

            if model2::fits(candidate, version, self.error_correction, self.fnc1, Some(header)) {
                return true;
            }
        }

        false
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
        force_initial_eci: bool,
    ) -> Result<Symbol, EncodeError> {
        self.encode_qr_range(
            self.versions.clone(),
            |version| {
                optimizer::text(
                    text,
                    optimizer::Profile::qr(version),
                    self.fnc1.is_some(),
                    force_initial_eci,
                )
            },
            structured_append,
        )
    }

    fn encode_qr_range<F>(
        &self,
        range: RangeInclusive<QrVersion>,
        mut segments: F,
        structured_append: Option<StructuredAppendInfo>,
    ) -> Result<Symbol, EncodeError>
    where
        F: FnMut(QrVersion) -> Result<Vec<Segment>, EncodeError>, {
        if range.start() > range.end() {
            return Err(EncodeError::InvalidVersionRange);
        }

        // Character count indicator widths change only at versions 10 and 27.
        let mut cache: [Option<Vec<Segment>>; 3] = [None, None, None];
        let mut last_error = None;

        for value in range.start().0..=range.end().0 {
            let version = QrVersion(value);
            let group = version_group(version);

            if cache[group].is_none() {
                cache[group] = Some(segments(version)?);
            }

            let candidate = cache[group].as_deref().expect("the version group is cached");

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
                Err(error) => last_error = Some(error),
            }
        }

        // The version range is verified to be non-empty, so at least one error was stored.
        Err(last_error.unwrap_or(EncodeError::InvalidVersionRange))
    }

    fn encode_segments_with_header(
        &self,
        segments: &[Segment],
        structured_append: Option<StructuredAppendInfo>,
    ) -> Result<Symbol, EncodeError> {
        let normalized;

        let segments = if self.fnc1.is_some() {
            normalized = segments
                .iter()
                .map(|segment| {
                    if segment.mode == Mode::Alphanumeric {
                        Segment::fnc1_alphanumeric(&segment.source)
                    } else {
                        Ok(segment.clone())
                    }
                })
                .collect::<Result<Vec<_>, _>>()?;
            normalized.as_slice()
        } else {
            segments
        };

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
        self.encode_optimized(|version| {
            optimizer::text(
                text,
                optimizer::Profile::rmqr(rmqr::cci(version)),
                self.fnc1.is_some(),
                false,
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
    pub fn encode_segments(&self, segments: &[Segment]) -> Result<Symbol, EncodeError> {
        let normalized;
        let segments = if self.fnc1.is_some() {
            normalized = segments
                .iter()
                .map(|segment| {
                    if segment.mode == Mode::Alphanumeric {
                        Segment::fnc1_alphanumeric(&segment.source)
                    } else {
                        Ok(segment.clone())
                    }
                })
                .collect::<Result<Vec<_>, _>>()?;
            normalized.as_slice()
        } else {
            segments
        };

        self.encode_optimized(|_| Ok(segments.to_vec()))
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

    fn encode_optimized<F>(&self, mut optimize: F) -> Result<Symbol, EncodeError>
    where
        F: FnMut(RmqrVersion) -> Result<Vec<Segment>, EncodeError>, {
        let versions = self.candidates()?;
        let mut cache: Vec<([u8; 4], Vec<Segment>)> = Vec::with_capacity(15);
        let mut last_error = None;

        for version in versions {
            let profile = rmqr::cci(version);
            let index = match cache.iter().position(|(cci, _)| *cci == profile) {
                Some(index) => index,
                None => {
                    cache.push((profile, optimize(version)?));
                    cache.len() - 1
                },
            };

            match rmqr::encode(
                &cache[index].1,
                version,
                self.error_correction,
                self.boost_error_correction,
                self.fnc1,
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
        if let Some((version, capacity, capacity_bits)) = self.input_capacity_upper_bound()?
            && micro_bytes_are_representable(data, version)
        {
            ensure_input_length(data.len(), capacity, capacity_bits, 1)?;
        }

        self.encode_range(|version| micro::optimize(data, version))
    }

    /// Encodes text with globally optimized modes supported by each candidate version.
    pub fn encode_text(&self, text: impl AsRef<str>) -> Result<Symbol, EncodeError> {
        let text = text.as_ref();
        if let Some((version, capacity, capacity_bits)) = self.input_capacity_upper_bound()?
            && micro_text_is_representable_without_kanji(text, version)
        {
            ensure_input_length(text.chars().count(), capacity, capacity_bits, 1)?;
        }

        self.encode_range(|version| micro::optimize_text(text, version))
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
    pub fn encode_segments(&self, segments: &[Segment]) -> Result<Symbol, EncodeError> {
        self.encode_range(|_| Ok(segments.to_vec()))
    }

    fn input_capacity_upper_bound(
        &self,
    ) -> Result<Option<(MicroVersion, usize, usize)>, EncodeError> {
        if self.versions.start() > self.versions.end() {
            return Err(EncodeError::InvalidVersionRange);
        }

        Ok(micro_versions(self.versions.clone())
            .filter_map(|version| {
                micro::input_capacity_upper_bound(version, self.error_correction)
                    .map(|(capacity, capacity_bits)| (version, capacity, capacity_bits))
            })
            .max_by_key(|&(_, capacity, _)| capacity))
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

/// Tries a configured Micro QR Code encoder before a configured Model 2 QR Code encoder.
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
        match self.micro.encode_bytes(data) {
            Ok(symbol) => Ok(symbol),
            Err(error) if candidate_rejection(&error) => self.qr.encode_bytes(data),
            Err(error) => Err(error),
        }
    }

    /// Encodes text using the smallest eligible symbol family.
    pub fn encode_text(&self, text: impl AsRef<str>) -> Result<Symbol, EncodeError> {
        let text = text.as_ref();
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
    end:      usize,
    area:     usize,
    previous: usize,
}

#[cfg(feature = "qr")]
fn minimum_area_partition<F>(
    length: usize,
    part_count: usize,
    range: RangeInclusive<QrVersion>,
    mut maximum_end: F,
) -> Result<Vec<(usize, usize)>, EncodeError>
where
    F: FnMut(usize, QrVersion) -> Option<usize>, {
    // Each layer represents one additional symbol in the fixed-size Structured Append sequence.
    let mut layers = vec![vec![PartitionState {
        end: 0, area: 0, previous: 0
    }]];

    for _ in 0..part_count {
        let previous_layer = layers.last().expect("the initial layer exists");
        let mut by_end = BTreeMap::new();

        for (previous, state) in previous_layer.iter().enumerate() {
            for value in range.start().value()..=range.end().value() {
                let version = QrVersion(value);

                let Some(end) = maximum_end(state.end, version) else {
                    continue;
                };

                let size = usize::from(version.value()) * 4 + 17;

                let candidate = PartitionState {
                    end,
                    area: state.area + size * size,
                    previous,
                };

                by_end
                    .entry(end)
                    .and_modify(|current: &mut PartitionState| {
                        if candidate.area < current.area {
                            *current = candidate;
                        }
                    })
                    .or_insert(candidate);
            }
        }

        let mut best_area = usize::MAX;
        let mut layer = Vec::new();

        for state in by_end.into_values().rev() {
            // A state that reaches farther with no greater area dominates every earlier state.
            if state.area < best_area {
                best_area = state.area;
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

    let mut state_index = layers[part_count].iter().position(|state| state.end == length).ok_or(
        EncodeError::DataTooLong {
            required_bits: None, capacity_bits: 0
        },
    )?;

    let mut result = Vec::with_capacity(part_count);

    for layer_index in (1..=part_count).rev() {
        let state = layers[layer_index][state_index];
        let previous = layers[layer_index - 1][state.previous];
        result.push((previous.end, state.end));
        state_index = state.previous;
    }

    result.reverse();

    Ok(result)
}

#[cfg(feature = "qr")]
fn maximum_fitting_end<F>(length: usize, start: usize, mut fits: F) -> Option<usize>
where
    F: FnMut(usize) -> bool, {
    if start == length {
        return None;
    }

    let mut low = start + 1;
    let mut high = length;
    let mut result = None;

    // Encoding fit is monotonic as the candidate slice grows from a fixed start.
    while low <= high {
        let middle = low + (high - low) / 2;

        if fits(middle) {
            result = Some(middle);
            low = middle + 1;
        } else {
            high = middle - 1;
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
fn micro_bytes_are_representable(data: &[u8], version: MicroVersion) -> bool {
    match version {
        MicroVersion::M1 => data.iter().all(u8::is_ascii_digit),
        MicroVersion::M2 => data.iter().all(|byte| alphanumeric_value(*byte).is_some()),
        MicroVersion::M3 | MicroVersion::M4 => true,
    }
}

#[cfg(feature = "micro-qr")]
fn micro_text_is_representable_without_kanji(text: &str, version: MicroVersion) -> bool {
    match version {
        MicroVersion::M1 => text.bytes().all(|byte| byte.is_ascii_digit()),
        MicroVersion::M2 => {
            text.is_ascii() && text.bytes().all(|byte| alphanumeric_value(byte).is_some())
        },
        MicroVersion::M3 | MicroVersion::M4 => {
            text.chars().all(|character| u32::from(character) <= 0xFF)
        },
    }
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

#[cfg(feature = "qr")]
fn text_boundaries(text: &str) -> Vec<usize> {
    let mut result: Vec<_> = text.char_indices().map(|(offset, _)| offset).collect();
    result.push(text.len());
    result
}

#[cfg(feature = "qr")]
#[inline]
fn requires_non_default_eci(text: &str) -> bool {
    text.chars().any(|character| u32::from(character) > 0xFF)
}
