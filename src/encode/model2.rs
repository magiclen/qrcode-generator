use alloc::{vec, vec::Vec};

use super::{
    Fnc1, Mode, QrErrorCorrection, QrMask, QrVersion, Segment, StructuredAppendInfo, Symbol,
    SymbolVersion, bits::BitBuffer, reed_solomon,
};
use crate::EncodeError;

const PENALTY_N1: i32 = 3;
const PENALTY_N2: i32 = 3;
const PENALTY_N3: i32 = 40;
const PENALTY_N4: i32 = 10;

pub(crate) fn encode(
    segments: &[Segment],
    version: QrVersion,
    mut error_correction: QrErrorCorrection,
    requested_mask: Option<QrMask>,
    boost_error_correction: bool,
    fnc1: Option<Fnc1>,
    structured_append: Option<StructuredAppendInfo>,
) -> Result<Symbol, EncodeError> {
    let used_bits = total_bits(segments, version, fnc1, structured_append)?;
    let mut capacity_bits = data_codewords(version, error_correction) * 8;

    if used_bits > capacity_bits {
        return Err(EncodeError::DataTooLong {
            required_bits: used_bits,
            capacity_bits,
        });
    }

    if boost_error_correction {
        for candidate in
            [QrErrorCorrection::Medium, QrErrorCorrection::Quartile, QrErrorCorrection::High]
        {
            let candidate_capacity = data_codewords(version, candidate) * 8;
            if candidate > error_correction && used_bits <= candidate_capacity {
                error_correction = candidate;
                capacity_bits = candidate_capacity;
            }
        }
    }

    let mut bits = BitBuffer::with_capacity(capacity_bits);

    if let Some(info) = structured_append {
        bits.append(0b0011, 4);
        bits.append(u32::from(info.index), 4);
        bits.append(u32::from(info.total - 1), 4);
        bits.append(u32::from(info.parity), 8);
    }

    // Leading ECI headers follow Structured Append but must precede an FNC1 header.
    let leading_eci = segments.iter().take_while(|segment| segment.mode == Mode::Eci).count();

    for segment in &segments[..leading_eci] {
        bits.append(u32::from(segment.mode.qr_bits()), 4);
        bits.extend(&segment.bits);
    }

    if let Some(fnc1) = fnc1 {
        match fnc1 {
            Fnc1::Gs1 => bits.append(0b0101, 4),
            Fnc1::Industry(indicator) => {
                bits.append(0b1001, 4);
                bits.append(u32::from(indicator.value()), 8);
            },
        }
    }

    for segment in &segments[leading_eci..] {
        bits.append(u32::from(segment.mode.qr_bits()), 4);
        if segment.mode != Mode::Eci {
            bits.append(segment.character_count as u32, segment.mode.cci_bits(version));
        }
        bits.extend(&segment.bits);
    }

    bits.append(0, (capacity_bits - bits.len()).min(4) as u8);

    while bits.len() & 7 != 0 {
        bits.push(false);
    }

    let mut pad = true;

    while bits.len() < capacity_bits {
        bits.append(if pad { 0xEC } else { 0x11 }, 8);
        pad = !pad;
    }

    let data = bits.into_bytes();
    let all_codewords = add_error_correction(&data, version, error_correction);
    let mut matrix = Matrix::new(version, error_correction);

    matrix.draw_codewords(&all_codewords);

    let mask = if let Some(mask) = requested_mask {
        mask.value()
    } else {
        // Format bits are redrawn for every candidate because they are part of the penalty score.
        let mut best_mask = 0;
        let mut best_penalty = i32::MAX;

        for candidate in 0..8 {
            matrix.apply_mask(candidate);
            matrix.draw_format(candidate);
            let penalty = matrix.penalty();
            if penalty < best_penalty {
                best_mask = candidate;
                best_penalty = penalty;
            }
            matrix.apply_mask(candidate);
        }

        best_mask
    };

    matrix.apply_mask(mask);
    matrix.draw_format(mask);

    Ok(Symbol {
        version: SymbolVersion::Qr(version),
        error_correction: error_correction.into(),
        mask,
        modules: matrix.modules,
        structured_append,
    })
}

// Reports whether the segments fit the version at the given error correction level.
pub(crate) fn fits(
    segments: &[Segment],
    version: QrVersion,
    error_correction: QrErrorCorrection,
    fnc1: Option<Fnc1>,
    structured_append: Option<StructuredAppendInfo>,
) -> bool {
    matches!(
        total_bits(segments, version, fnc1, structured_append),
        Ok(used_bits) if used_bits <= data_codewords(version, error_correction) * 8
    )
}

fn total_bits(
    segments: &[Segment],
    version: QrVersion,
    fnc1: Option<Fnc1>,
    structured_append: Option<StructuredAppendInfo>,
) -> Result<usize, EncodeError> {
    let mut result = usize::from(structured_append.is_some()) * 20;

    result += match fnc1 {
        Some(Fnc1::Gs1) => 4,
        Some(Fnc1::Industry(_)) => 12,
        None => 0,
    };

    for segment in segments {
        let cci = segment.mode.cci_bits(version);

        if segment.mode != Mode::Eci && segment.character_count >= 1usize << cci {
            return Err(EncodeError::DataTooLong {
                required_bits: usize::MAX,
                capacity_bits: data_codewords(version, QrErrorCorrection::Low) * 8,
            });
        }

        result = result.checked_add(4 + usize::from(cci) + segment.bits.len()).ok_or(
            EncodeError::DataTooLong {
                required_bits: usize::MAX, capacity_bits: 0
            },
        )?;
    }
    Ok(result)
}

fn add_error_correction(
    data: &[u8],
    version: QrVersion,
    error_correction: QrErrorCorrection,
) -> Vec<u8> {
    let ordinal = error_correction.qr_ordinal();
    let version_index = usize::from(version.value());
    let block_count = NUM_ERROR_CORRECTION_BLOCKS[ordinal][version_index] as usize;
    let ecc_length = ECC_CODEWORDS_PER_BLOCK[ordinal][version_index] as usize;
    let raw_codewords = raw_data_modules(version) / 8;
    let short_block_count = block_count - raw_codewords % block_count;
    let short_block_length = raw_codewords / block_count;
    let divisor = reed_solomon::divisor(ecc_length);
    let mut blocks = Vec::with_capacity(block_count);
    let mut offset = 0;

    for index in 0..block_count {
        let data_length = short_block_length - ecc_length + usize::from(index >= short_block_count);
        let mut block = data[offset..offset + data_length].to_vec();

        offset += data_length;

        let ecc = reed_solomon::remainder(&block, &divisor);

        // A dummy byte aligns short blocks with long blocks during column interleaving.
        if index < short_block_count {
            block.push(0);
        }

        block.extend(ecc);
        blocks.push(block);
    }

    debug_assert_eq!(offset, data.len());

    // Data columns are emitted first and equal-length ECC columns follow them.
    let mut result = Vec::with_capacity(raw_codewords);

    for column in 0..=short_block_length {
        for (block_index, block) in blocks.iter().enumerate() {
            if column != short_block_length - ecc_length || block_index >= short_block_count {
                result.push(block[column]);
            }
        }
    }

    debug_assert_eq!(result.len(), raw_codewords);

    result
}

#[inline]
pub(crate) const fn data_codewords(
    version: QrVersion,
    error_correction: QrErrorCorrection,
) -> usize {
    let ordinal = error_correction.qr_ordinal();
    let version_index = version.value() as usize;
    raw_data_modules(version) / 8
        - ECC_CODEWORDS_PER_BLOCK[ordinal][version_index] as usize
            * NUM_ERROR_CORRECTION_BLOCKS[ordinal][version_index] as usize
}

#[inline]
const fn raw_data_modules(version: QrVersion) -> usize {
    let version = version.value() as usize;
    let mut result = (16 * version + 128) * version + 64;

    if version >= 2 {
        let alignment_count = version / 7 + 2;

        result -= (25 * alignment_count - 10) * alignment_count - 55;
        if version >= 7 {
            result -= 36;
        }
    }

    result
}

struct Matrix {
    version:          QrVersion,
    error_correction: QrErrorCorrection,
    size:             usize,
    modules:          Vec<bool>,
    function:         Vec<bool>,
}

impl Matrix {
    fn new(version: QrVersion, error_correction: QrErrorCorrection) -> Self {
        let size = usize::from(version.value()) * 4 + 17;

        let mut result = Self {
            version,
            error_correction,
            size,
            modules: vec![false; size * size],
            function: vec![false; size * size],
        };

        result.draw_function_patterns();
        result
    }

    fn draw_function_patterns(&mut self) {
        for coordinate in 0..self.size {
            self.set_function(6, coordinate, coordinate % 2 == 0);
            self.set_function(coordinate, 6, coordinate % 2 == 0);
        }

        self.draw_finder(3, 3);
        self.draw_finder(self.size as isize - 4, 3);
        self.draw_finder(3, self.size as isize - 4);

        let positions = alignment_positions(self.version);

        for (row, &y) in positions.iter().enumerate() {
            for (column, &x) in positions.iter().enumerate() {
                let last = positions.len() - 1;

                if !((row == 0 && column == 0)
                    || (row == 0 && column == last)
                    || (row == last && column == 0))
                {
                    self.draw_alignment(x, y);
                }
            }
        }

        // Drawing placeholder format and version bits reserves them before data placement.
        self.draw_format(0);
        self.draw_version();
    }

    fn draw_finder(&mut self, center_x: isize, center_y: isize) {
        for dy in -4isize..=4 {
            for dx in -4isize..=4 {
                let x = center_x + dx;
                let y = center_y + dy;

                if x >= 0 && y >= 0 && x < self.size as isize && y < self.size as isize {
                    let distance = dx.abs().max(dy.abs());

                    self.set_function(x as usize, y as usize, distance != 2 && distance != 4);
                }
            }
        }
    }

    fn draw_alignment(&mut self, center_x: usize, center_y: usize) {
        for dy in -2isize..=2 {
            for dx in -2isize..=2 {
                self.set_function(
                    (center_x as isize + dx) as usize,
                    (center_y as isize + dy) as usize,
                    dx.abs().max(dy.abs()) != 1,
                );
            }
        }
    }

    fn draw_format(&mut self, mask: u8) {
        let bits = format_bits(self.error_correction, mask);

        for index in 0..6 {
            self.set_function(8, index, bit(bits, index));
        }

        self.set_function(8, 7, bit(bits, 6));
        self.set_function(8, 8, bit(bits, 7));
        self.set_function(7, 8, bit(bits, 8));

        for index in 9..15 {
            self.set_function(14 - index, 8, bit(bits, index));
        }

        for index in 0..8 {
            self.set_function(self.size - 1 - index, 8, bit(bits, index));
        }

        for index in 8..15 {
            self.set_function(8, self.size - 15 + index, bit(bits, index));
        }

        self.set_function(8, self.size - 8, true);
    }

    fn draw_version(&mut self) {
        if self.version.value() < 7 {
            return;
        }

        let bits = version_bits(self.version);

        for index in 0..18 {
            let a = self.size - 11 + index % 3;
            let b = index / 3;

            self.set_function(a, b, bit(bits, index));
            self.set_function(b, a, bit(bits, index));
        }
    }

    fn draw_codewords(&mut self, data: &[u8]) {
        // Data travels in alternating two-column stripes and skips every reserved module.
        let mut bit_index = 0;
        let mut right = self.size - 1;

        while right >= 1 {
            if right == 6 {
                right = 5;
            }

            for vertical in 0..self.size {
                for offset in 0..2 {
                    let x = right - offset;
                    let upward = (right + 1) & 2 == 0;

                    let y = if upward { self.size - 1 - vertical } else { vertical };

                    let index = y * self.size + x;

                    if !self.function[index] && bit_index < data.len() * 8 {
                        self.modules[index] =
                            data[bit_index >> 3] >> (7 - (bit_index & 7)) & 1 != 0;
                        bit_index += 1;
                    }
                }
            }

            if right < 2 {
                break;
            }

            right -= 2;
        }
        debug_assert_eq!(bit_index, data.len() * 8);
    }

    fn apply_mask(&mut self, mask: u8) {
        for y in 0..self.size {
            for x in 0..self.size {
                let invert = match mask {
                    0 => (x + y) % 2 == 0,
                    1 => y % 2 == 0,
                    2 => x % 3 == 0,
                    3 => (x + y) % 3 == 0,
                    4 => (x / 3 + y / 2) % 2 == 0,
                    5 => x * y % 2 + x * y % 3 == 0,
                    6 => (x * y % 2 + x * y % 3) % 2 == 0,
                    7 => ((x + y) % 2 + x * y % 3) % 2 == 0,
                    _ => unreachable!(),
                };

                let index = y * self.size + x;

                if invert && !self.function[index] {
                    self.modules[index] = !self.modules[index];
                }
            }
        }
    }

    fn penalty(&self) -> i32 {
        let mut score = 0;

        for y in 0..self.size {
            score += line_penalty(&self.modules[y * self.size..][..self.size]);
        }

        // The reused column buffer avoids one allocation per strided column line.
        let mut column = Vec::with_capacity(self.size);

        for x in 0..self.size {
            column.clear();
            column.extend((0..self.size).map(|y| self.modules[y * self.size + x]));
            score += line_penalty(&column);
        }

        for y in 0..self.size - 1 {
            for x in 0..self.size - 1 {
                let value = self.modules[y * self.size + x];
                if value == self.modules[y * self.size + x + 1]
                    && value == self.modules[(y + 1) * self.size + x]
                    && value == self.modules[(y + 1) * self.size + x + 1]
                {
                    score += PENALTY_N2;
                }
            }
        }

        let dark = self.modules.iter().filter(|&&value| value).count();

        score + n4_penalty(dark, self.size * self.size)
    }

    #[inline]
    fn set_function(&mut self, x: usize, y: usize, value: bool) {
        let index = y * self.size + x;

        self.modules[index] = value;
        self.function[index] = true;
    }
}

fn format_bits(error_correction: QrErrorCorrection, mask: u8) -> u32 {
    let data = u32::from(error_correction.qr_format_bits() << 3 | mask);
    let mut remainder = data;

    for _ in 0..10 {
        remainder = (remainder << 1) ^ ((remainder >> 9) * 0x537);
    }

    (data << 10 | remainder) ^ 0x5412
}

fn version_bits(version: QrVersion) -> u32 {
    let data = u32::from(version.value());
    let mut remainder = data;

    for _ in 0..12 {
        remainder = (remainder << 1) ^ ((remainder >> 11) * 0x1F25);
    }

    data << 12 | remainder
}

fn line_penalty(values: &[bool]) -> i32 {
    let mut score = 0;
    let mut run = 1;

    for index in 1..values.len() {
        if values[index] == values[index - 1] {
            run += 1;
            if run == 5 {
                score += PENALTY_N1;
            } else if run > 5 {
                score += 1;
            }
        } else {
            run = 1;
        }
    }

    for window in values.windows(11) {
        if window == [true, false, true, true, true, false, true, false, false, false, false]
            || window == [false, false, false, false, true, false, true, true, true, false, true]
        {
            score += PENALTY_N3;
        }
    }
    score
}

// The 45% and 55% dark ratios stay in the zero-penalty band, matching an inclusive reading of NOTE 4.
// A real symbol has an odd module count, so its dark ratio never lands on those endpoints exactly.
fn n4_penalty(dark: usize, total: usize) -> i32 {
    let dark = dark as i32;
    let total = total as i32;
    let deviation = ((dark * 20 - total * 10).abs() + total - 1) / total - 1;

    deviation * PENALTY_N4
}

fn alignment_positions(version: QrVersion) -> Vec<usize> {
    if version.value() == 1 {
        return Vec::new();
    }

    let count = usize::from(version.value()) / 7 + 2;
    let size = usize::from(version.value()) * 4 + 17;

    // Version 32 is the only alignment layout that cannot use the general spacing formula.
    let step = if version.value() == 32 {
        26
    } else {
        (usize::from(version.value()) * 4 + count * 2 + 1) / (count * 2 - 2) * 2
    };

    let mut result: Vec<_> = (0..count - 1).map(|index| size - 7 - index * step).collect();

    result.push(6);
    result.reverse();
    result
}

#[inline]
const fn bit(value: u32, index: usize) -> bool {
    value >> index & 1 != 0
}

static ECC_CODEWORDS_PER_BLOCK: [[i8; 41]; 4] = [
    [
        -1, 7, 10, 15, 20, 26, 18, 20, 24, 30, 18, 20, 24, 26, 30, 22, 24, 28, 30, 28, 28, 28, 28,
        30, 30, 26, 28, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30,
    ],
    [
        -1, 10, 16, 26, 18, 24, 16, 18, 22, 22, 26, 30, 22, 22, 24, 24, 28, 28, 26, 26, 26, 26, 28,
        28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28,
    ],
    [
        -1, 13, 22, 18, 26, 18, 24, 18, 22, 20, 24, 28, 26, 24, 20, 30, 24, 28, 28, 26, 30, 28, 30,
        30, 30, 30, 28, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30,
    ],
    [
        -1, 17, 28, 22, 16, 22, 28, 26, 26, 24, 28, 24, 28, 22, 24, 24, 30, 28, 28, 26, 28, 30, 24,
        30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30,
    ],
];

static NUM_ERROR_CORRECTION_BLOCKS: [[i8; 41]; 4] = [
    [
        -1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 4, 4, 4, 4, 4, 6, 6, 6, 6, 7, 8, 8, 9, 9, 10, 12, 12, 12,
        13, 14, 15, 16, 17, 18, 19, 19, 20, 21, 22, 24, 25,
    ],
    [
        -1, 1, 1, 1, 2, 2, 4, 4, 4, 5, 5, 5, 8, 9, 9, 10, 10, 11, 13, 14, 16, 17, 17, 18, 20, 21,
        23, 25, 26, 28, 29, 31, 33, 35, 37, 38, 40, 43, 45, 47, 49,
    ],
    [
        -1, 1, 1, 2, 2, 4, 4, 6, 6, 8, 8, 8, 10, 12, 16, 12, 17, 16, 18, 21, 20, 23, 23, 25, 27,
        29, 34, 34, 35, 38, 40, 43, 45, 48, 51, 53, 56, 59, 62, 65, 68,
    ],
    [
        -1, 1, 1, 2, 4, 4, 4, 5, 6, 8, 8, 11, 11, 16, 16, 18, 16, 19, 21, 25, 25, 25, 34, 30, 32,
        35, 37, 40, 42, 45, 48, 51, 54, 57, 60, 63, 66, 70, 74, 77, 81,
    ],
];
