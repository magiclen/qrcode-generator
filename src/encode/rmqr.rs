use alloc::{vec, vec::Vec};

use super::{
    Fnc1, Mode, RmqrErrorCorrection, RmqrVersion, Segment, Symbol, SymbolVersion, bits::BitBuffer,
    reed_solomon,
};
use crate::EncodeError;

#[derive(Clone, Copy)]
struct BlockGroup {
    count: u8,
    total: u8,
    data:  u8,
}

#[derive(Clone, Copy)]
struct EcInfo {
    data_codewords: u16,
    groups:         [BlockGroup; 2],
}

#[derive(Clone, Copy)]
pub(crate) struct VersionInfo {
    total_codewords: u16,
    remainder_bits:  u8,
    cci:             [u8; 4],
    medium:          EcInfo,
    high:            EcInfo,
}

const EMPTY_GROUP: BlockGroup = BlockGroup {
    count: 0, total: 0, data: 0
};

const fn group(count: u8, total: u8, data: u8) -> BlockGroup {
    BlockGroup {
        count,
        total,
        data,
    }
}

const fn ec(data_codewords: u16, first: BlockGroup, second: BlockGroup) -> EcInfo {
    EcInfo {
        data_codewords,
        groups: [first, second],
    }
}

const fn info(
    total_codewords: u16,
    remainder_bits: u8,
    cci: [u8; 4],
    medium: EcInfo,
    high: EcInfo,
) -> VersionInfo {
    VersionInfo {
        total_codewords,
        remainder_bits,
        cci,
        medium,
        high,
    }
}

const VERSION_INFO: [VersionInfo; 32] = [
    info(
        13,
        0,
        [4, 3, 3, 2],
        ec(6, group(1, 13, 6), EMPTY_GROUP),
        ec(3, group(1, 13, 3), EMPTY_GROUP),
    ),
    info(
        21,
        3,
        [5, 5, 4, 3],
        ec(12, group(1, 21, 12), EMPTY_GROUP),
        ec(7, group(1, 21, 7), EMPTY_GROUP),
    ),
    info(
        32,
        5,
        [6, 5, 5, 4],
        ec(20, group(1, 32, 20), EMPTY_GROUP),
        ec(10, group(1, 32, 10), EMPTY_GROUP),
    ),
    info(
        44,
        6,
        [7, 6, 5, 5],
        ec(28, group(1, 44, 28), EMPTY_GROUP),
        ec(14, group(1, 44, 14), EMPTY_GROUP),
    ),
    info(
        68,
        1,
        [7, 6, 6, 5],
        ec(44, group(1, 68, 44), EMPTY_GROUP),
        ec(24, group(2, 34, 12), EMPTY_GROUP),
    ),
    info(
        21,
        2,
        [5, 5, 4, 3],
        ec(12, group(1, 21, 12), EMPTY_GROUP),
        ec(7, group(1, 21, 7), EMPTY_GROUP),
    ),
    info(
        33,
        3,
        [6, 5, 5, 4],
        ec(21, group(1, 33, 21), EMPTY_GROUP),
        ec(11, group(1, 33, 11), EMPTY_GROUP),
    ),
    info(
        49,
        1,
        [7, 6, 5, 5],
        ec(31, group(1, 49, 31), EMPTY_GROUP),
        ec(17, group(1, 24, 8), group(1, 25, 9)),
    ),
    info(
        66,
        4,
        [7, 6, 6, 5],
        ec(42, group(1, 66, 42), EMPTY_GROUP),
        ec(22, group(2, 33, 11), EMPTY_GROUP),
    ),
    info(
        99,
        5,
        [8, 7, 6, 6],
        ec(63, group(1, 49, 31), group(1, 50, 32)),
        ec(33, group(3, 33, 11), EMPTY_GROUP),
    ),
    info(
        15,
        2,
        [4, 4, 3, 2],
        ec(7, group(1, 15, 7), EMPTY_GROUP),
        ec(5, group(1, 15, 5), EMPTY_GROUP),
    ),
    info(
        31,
        1,
        [6, 5, 5, 4],
        ec(19, group(1, 31, 19), EMPTY_GROUP),
        ec(11, group(1, 31, 11), EMPTY_GROUP),
    ),
    info(
        47,
        0,
        [7, 6, 5, 5],
        ec(31, group(1, 47, 31), EMPTY_GROUP),
        ec(15, group(1, 23, 7), group(1, 24, 8)),
    ),
    info(
        67,
        2,
        [7, 6, 6, 5],
        ec(43, group(1, 67, 43), EMPTY_GROUP),
        ec(23, group(1, 33, 11), group(1, 34, 12)),
    ),
    info(
        89,
        7,
        [8, 7, 6, 6],
        ec(57, group(1, 44, 28), group(1, 45, 29)),
        ec(29, group(1, 44, 14), group(1, 45, 15)),
    ),
    info(
        132,
        6,
        [8, 7, 7, 6],
        ec(84, group(2, 66, 42), EMPTY_GROUP),
        ec(42, group(3, 44, 14), EMPTY_GROUP),
    ),
    info(
        21,
        4,
        [5, 5, 4, 3],
        ec(12, group(1, 21, 12), EMPTY_GROUP),
        ec(7, group(1, 21, 7), EMPTY_GROUP),
    ),
    info(
        41,
        1,
        [6, 6, 5, 5],
        ec(27, group(1, 41, 27), EMPTY_GROUP),
        ec(13, group(1, 41, 13), EMPTY_GROUP),
    ),
    info(
        60,
        6,
        [7, 6, 6, 5],
        ec(38, group(1, 60, 38), EMPTY_GROUP),
        ec(20, group(2, 30, 10), EMPTY_GROUP),
    ),
    info(
        85,
        4,
        [7, 7, 6, 6],
        ec(53, group(1, 42, 26), group(1, 43, 27)),
        ec(29, group(1, 42, 14), group(1, 43, 15)),
    ),
    info(
        113,
        3,
        [8, 7, 7, 6],
        ec(73, group(1, 56, 36), group(1, 57, 37)),
        ec(35, group(1, 37, 11), group(2, 38, 12)),
    ),
    info(
        166,
        0,
        [8, 8, 7, 7],
        ec(106, group(2, 55, 35), group(1, 56, 36)),
        ec(54, group(2, 41, 13), group(2, 42, 14)),
    ),
    info(
        51,
        1,
        [7, 6, 6, 5],
        ec(33, group(1, 51, 33), EMPTY_GROUP),
        ec(15, group(1, 25, 7), group(1, 26, 8)),
    ),
    info(
        74,
        4,
        [7, 7, 6, 5],
        ec(48, group(1, 74, 48), EMPTY_GROUP),
        ec(26, group(2, 37, 13), EMPTY_GROUP),
    ),
    info(
        103,
        6,
        [8, 7, 7, 6],
        ec(67, group(1, 51, 33), group(1, 52, 34)),
        ec(31, group(2, 34, 10), group(1, 35, 11)),
    ),
    info(
        136,
        7,
        [8, 7, 7, 6],
        ec(88, group(2, 68, 44), EMPTY_GROUP),
        ec(48, group(4, 34, 12), EMPTY_GROUP),
    ),
    info(
        199,
        2,
        [9, 8, 7, 7],
        ec(127, group(2, 66, 42), group(1, 67, 43)),
        ec(69, group(1, 39, 13), group(4, 40, 14)),
    ),
    info(
        61,
        1,
        [7, 6, 6, 5],
        ec(39, group(1, 61, 39), EMPTY_GROUP),
        ec(21, group(1, 30, 10), group(1, 31, 11)),
    ),
    info(
        88,
        2,
        [8, 7, 6, 6],
        ec(56, group(2, 44, 28), EMPTY_GROUP),
        ec(28, group(2, 44, 14), EMPTY_GROUP),
    ),
    info(
        122,
        0,
        [8, 7, 7, 6],
        ec(78, group(2, 61, 39), EMPTY_GROUP),
        ec(38, group(1, 40, 12), group(2, 41, 13)),
    ),
    info(
        160,
        3,
        [8, 8, 7, 6],
        ec(100, group(2, 53, 33), group(1, 54, 34)),
        ec(56, group(4, 40, 14), EMPTY_GROUP),
    ),
    info(
        232,
        4,
        [9, 8, 8, 7],
        ec(152, group(4, 58, 38), EMPTY_GROUP),
        ec(76, group(2, 38, 12), group(4, 39, 13)),
    ),
];

#[inline]
pub(crate) const fn version_info(version: RmqrVersion) -> &'static VersionInfo {
    &VERSION_INFO[version.value() as usize]
}

#[inline]
pub(crate) const fn cci(version: RmqrVersion) -> [u8; 4] {
    version_info(version).cci
}

#[inline]
pub(crate) const fn data_codewords(
    version: RmqrVersion,
    error_correction: RmqrErrorCorrection,
) -> usize {
    ec_info(version, error_correction).data_codewords as usize
}

#[inline]
const fn ec_info(version: RmqrVersion, error_correction: RmqrErrorCorrection) -> &'static EcInfo {
    let info = version_info(version);

    match error_correction {
        RmqrErrorCorrection::Medium => &info.medium,
        RmqrErrorCorrection::High => &info.high,
    }
}

pub(crate) fn encode(
    segments: &[Segment],
    version: RmqrVersion,
    mut error_correction: RmqrErrorCorrection,
    boost_error_correction: bool,
    fnc1: Option<Fnc1>,
) -> Result<Symbol, EncodeError> {
    let mut capacity_bits = data_codewords(version, error_correction) * 8;
    let used_bits = total_bits(segments, version, error_correction, fnc1)?;

    if used_bits > capacity_bits {
        return Err(EncodeError::DataTooLong {
            required_bits: used_bits,
            capacity_bits,
        });
    }

    if boost_error_correction && error_correction == RmqrErrorCorrection::Medium {
        let high_capacity = data_codewords(version, RmqrErrorCorrection::High) * 8;

        if used_bits <= high_capacity {
            error_correction = RmqrErrorCorrection::High;
            capacity_bits = high_capacity;
        }
    }

    let mut bits = BitBuffer::with_capacity(capacity_bits);
    let leading_eci = segments.iter().take_while(|segment| segment.mode == Mode::Eci).count();

    for segment in &segments[..leading_eci] {
        bits.append(u32::from(mode_bits(segment.mode)), 3);
        bits.extend(&segment.bits);
    }

    if let Some(fnc1) = fnc1 {
        match fnc1 {
            Fnc1::Gs1 => bits.append(0b101, 3),
            Fnc1::Industry(indicator) => {
                bits.append(0b110, 3);
                bits.append(u32::from(indicator.value()), 8);
            },
        }
    }

    let cci = version_info(version).cci;

    for segment in &segments[leading_eci..] {
        bits.append(u32::from(mode_bits(segment.mode)), 3);

        if segment.mode != Mode::Eci {
            bits.append(segment.character_count as u32, cci_bits(cci, segment.mode));
        }

        bits.extend(&segment.bits);
    }

    bits.append(0, (capacity_bits - bits.len()).min(3) as u8);

    while bits.len() & 7 != 0 {
        bits.push(false);
    }

    let mut pad = true;

    while bits.len() < capacity_bits {
        bits.append(if pad { 0xEC } else { 0x11 }, 8);
        pad = !pad;
    }

    let data = bits.into_bytes();
    let message = final_message(&data, version, error_correction);
    let mut matrix = Matrix::new(version, error_correction);

    matrix.draw_codewords(&message);
    matrix.draw_format();

    Ok(Symbol {
        version:                                  SymbolVersion::Rmqr(version),
        error_correction:                         error_correction.into(),
        mask:                                     0,
        modules:                                  matrix.modules,
        #[cfg(feature = "qr")]
        structured_append:                        None,
    })
}

fn total_bits(
    segments: &[Segment],
    version: RmqrVersion,
    error_correction: RmqrErrorCorrection,
    fnc1: Option<Fnc1>,
) -> Result<usize, EncodeError> {
    let cci = version_info(version).cci;
    let mut result: usize = match fnc1 {
        Some(Fnc1::Gs1) => 3,
        Some(Fnc1::Industry(_)) => 11,
        None => 0,
    };

    for segment in segments {
        let count_bits = cci_bits(cci, segment.mode);

        if segment.mode != Mode::Eci && segment.character_count >= 1usize << count_bits {
            return Err(EncodeError::DataTooLong {
                required_bits: usize::MAX,
                capacity_bits: data_codewords(version, error_correction) * 8,
            });
        }

        result = result.checked_add(3 + usize::from(count_bits) + segment.bits.len()).ok_or(
            EncodeError::DataTooLong {
                required_bits: usize::MAX,
                capacity_bits: data_codewords(version, error_correction) * 8,
            },
        )?;
    }

    Ok(result)
}

#[inline]
const fn mode_bits(mode: Mode) -> u8 {
    match mode {
        Mode::Numeric => 0b001,
        Mode::Alphanumeric => 0b010,
        Mode::Byte => 0b011,
        Mode::Kanji => 0b100,
        Mode::Eci => 0b111,
    }
}

#[inline]
const fn cci_bits(cci: [u8; 4], mode: Mode) -> u8 {
    match mode {
        Mode::Numeric => cci[0],
        Mode::Alphanumeric => cci[1],
        Mode::Byte => cci[2],
        Mode::Kanji => cci[3],
        Mode::Eci => 0,
    }
}

fn final_message(
    data: &[u8],
    version: RmqrVersion,
    error_correction: RmqrErrorCorrection,
) -> BitBuffer {
    let info = version_info(version);
    let ec = ec_info(version, error_correction);
    let mut blocks = Vec::new();
    let mut offset = 0;

    for group in ec.groups {
        for _ in 0..group.count {
            let data_length = group.data as usize;
            let block_data = data[offset..offset + data_length].to_vec();
            let divisor = reed_solomon::divisor((group.total - group.data) as usize);
            let ecc = reed_solomon::remainder(&block_data, &divisor);

            blocks.push((block_data, ecc));
            offset += data_length;
        }
    }

    debug_assert_eq!(offset, data.len());

    let mut codewords = Vec::with_capacity(info.total_codewords as usize);
    let maximum_data = blocks.iter().map(|(data, _)| data.len()).max().unwrap_or(0);

    for column in 0..maximum_data {
        for (block_data, _) in &blocks {
            if let Some(&byte) = block_data.get(column) {
                codewords.push(byte);
            }
        }
    }

    let maximum_ecc = blocks.iter().map(|(_, ecc)| ecc.len()).max().unwrap_or(0);

    for column in 0..maximum_ecc {
        for (_, ecc) in &blocks {
            if let Some(&byte) = ecc.get(column) {
                codewords.push(byte);
            }
        }
    }

    debug_assert_eq!(codewords.len(), info.total_codewords as usize);

    let mut result = BitBuffer::with_capacity(codewords.len() * 8 + info.remainder_bits as usize);

    for byte in codewords {
        result.append(u32::from(byte), 8);
    }

    result.append(0, info.remainder_bits);
    result
}

struct Matrix {
    version:          RmqrVersion,
    error_correction: RmqrErrorCorrection,
    width:            usize,
    height:           usize,
    modules:          Vec<bool>,
    function:         Vec<bool>,
}

impl Matrix {
    fn new(version: RmqrVersion, error_correction: RmqrErrorCorrection) -> Self {
        let width = version.width();
        let height = version.height();
        let mut result = Self {
            version,
            error_correction,
            width,
            height,
            modules: vec![false; width * height],
            function: vec![false; width * height],
        };

        result.draw_function_patterns();
        result
    }

    fn draw_function_patterns(&mut self) {
        for row in 0usize..7 {
            for column in 0usize..7 {
                let distance = row.abs_diff(3).max(column.abs_diff(3));
                self.set_function(column, row, distance != 2);
            }
        }

        for row in 0..self.height.min(8) {
            self.set_function(7, row, false);
        }

        if self.height > 7 {
            for column in 0..8 {
                self.set_function(column, 7, false);
            }
        }

        let sub_x = self.width - 5;
        let sub_y = self.height - 5;

        for row in 0..5 {
            for column in 0..5 {
                self.set_function(
                    sub_x + column,
                    sub_y + row,
                    row == 0 || row == 4 || column == 0 || column == 4 || (row == 2 && column == 2),
                );
            }
        }

        self.set_function(self.width - 2, 0, true);
        self.set_function(self.width - 1, 0, true);
        self.set_function(self.width - 2, 1, false);
        self.set_function(self.width - 1, 1, true);

        if self.height >= 11 {
            self.set_function(0, self.height - 2, true);
            self.set_function(1, self.height - 2, false);
        }

        self.set_function(0, self.height - 1, true);
        self.set_function(1, self.height - 1, true);
        self.set_function(2, self.height - 1, true);

        for center in alignment_centers(self.width) {
            for row in 0..3 {
                for offset in 0..3 {
                    let value = row != 1 || offset != 1;
                    self.set_function(center + offset - 1, row, value);
                    self.set_function(center + offset - 1, self.height - 3 + row, value);
                }
            }
        }

        for column in 0..self.width {
            if !self.is_function(column, 0) {
                self.set_function(column, 0, column & 1 == 0);
            }

            if !self.is_function(column, self.height - 1) {
                self.set_function(column, self.height - 1, column & 1 == 0);
            }
        }

        let mut timing_columns = Vec::with_capacity(2 + alignment_centers(self.width).len());

        timing_columns.push(0);
        timing_columns.extend_from_slice(alignment_centers(self.width));
        timing_columns.push(self.width - 1);

        for column in timing_columns {
            for row in 0..self.height {
                if !self.is_function(column, row) {
                    self.set_function(column, row, row & 1 == 0);
                }
            }
        }

        for (column, row) in finder_format_coordinates() {
            self.set_function(column, row, false);
        }

        for (column, row) in sub_format_coordinates(self.width, self.height) {
            self.set_function(column, row, false);
        }
    }

    fn draw_codewords(&mut self, bits: &BitBuffer) {
        let mut bit_index = 0;
        let mut column = self.width - 2;
        let mut row = self.height - 6;
        let mut upward = true;

        loop {
            for current_column in [column, column - 1] {
                if !self.is_function(current_column, row) {
                    let mut value = bits.bit(bit_index);

                    if ((row / 2) + (current_column / 3)) & 1 == 0 {
                        value = !value;
                    }

                    self.modules[row * self.width + current_column] = value;
                    bit_index += 1;
                }
            }

            if upward {
                if row == 1 {
                    if column < 2 {
                        break;
                    }
                    column -= 2;
                    upward = false;
                } else {
                    row -= 1;
                }
            } else if row == self.height - 2 {
                if column < 2 {
                    break;
                }
                column -= 2;
                upward = true;
            } else {
                row += 1;
            }
        }

        debug_assert_eq!(bit_index, bits.len());
    }

    fn draw_format(&mut self) {
        let data = (u32::from(self.error_correction == RmqrErrorCorrection::High) << 5)
            | u32::from(self.version.value());
        let raw = format_code(data);
        let finder = raw ^ 0b011111101010110010;
        let sub = raw ^ 0b100000101001111011;

        for (index, (column, row)) in finder_format_coordinates().enumerate() {
            self.modules[row * self.width + column] = finder >> index & 1 != 0;
        }

        for (index, (column, row)) in sub_format_coordinates(self.width, self.height).enumerate() {
            self.modules[row * self.width + column] = sub >> index & 1 != 0;
        }
    }

    #[inline]
    fn is_function(&self, column: usize, row: usize) -> bool {
        self.function[row * self.width + column]
    }

    #[inline]
    fn set_function(&mut self, column: usize, row: usize, value: bool) {
        let index = row * self.width + column;

        self.modules[index] = value;
        self.function[index] = true;
    }
}

#[inline]
fn alignment_centers(width: usize) -> &'static [usize] {
    match width {
        27 => &[],
        43 => &[21],
        59 => &[19, 39],
        77 => &[25, 51],
        99 => &[23, 49, 75],
        139 => &[27, 55, 83, 111],
        _ => unreachable!("rMQR width is validated by RmqrVersion"),
    }
}

fn finder_format_coordinates() -> impl Iterator<Item = (usize, usize)> {
    (0..18).map(|index| match index {
        0..=4 => (8, index + 1),
        5..=9 => (9, index - 4),
        10..=14 => (10, index - 9),
        _ => (11, index - 14),
    })
}

fn sub_format_coordinates(width: usize, height: usize) -> impl Iterator<Item = (usize, usize)> {
    let anchor_x = width - 8;
    let anchor_y = height - 6;

    (0..18).map(move |index| match index {
        0..=4 => (anchor_x, anchor_y + index),
        5..=9 => (anchor_x + 1, anchor_y + index - 5),
        10..=14 => (anchor_x + 2, anchor_y + index - 10),
        _ => (anchor_x + index - 12, anchor_y),
    })
}

fn format_code(data: u32) -> u32 {
    let mut value = data << 12;

    for bit in (12..=17).rev() {
        if value >> bit & 1 != 0 {
            value ^= 0x1F25 << (bit - 12);
        }
    }

    data << 12 | value
}

#[cfg(test)]
mod tests {
    use alloc::string::String;

    use super::*;

    #[test]
    fn every_version_table_and_geometry_entry_is_consistent() {
        for version in RmqrVersion::ALL {
            let info = version_info(version);
            let matrix = Matrix::new(version, RmqrErrorCorrection::Medium);
            let data_modules = matrix.function.iter().filter(|&&value| !value).count();

            assert_eq!(
                data_modules,
                info.total_codewords as usize * 8 + info.remainder_bits as usize,
                "{version:?}",
            );

            for error_correction in [RmqrErrorCorrection::Medium, RmqrErrorCorrection::High] {
                let ec = ec_info(version, error_correction);
                let total: usize =
                    ec.groups.iter().map(|group| group.count as usize * group.total as usize).sum();
                let data: usize =
                    ec.groups.iter().map(|group| group.count as usize * group.data as usize).sum();

                assert_eq!(total, info.total_codewords as usize, "{version:?}");
                assert_eq!(data, ec.data_codewords as usize, "{version:?}");
            }
        }
    }

    #[test]
    fn annex_i_codewords_and_matrix() {
        let data = [0x2E, 0x06, 0x2B, 0x2C, 0x00];
        let message = final_message(&data, RmqrVersion::R11x27, RmqrErrorCorrection::High);
        let expected = [
            0x2E, 0x06, 0x2B, 0x2C, 0x00, 0x11, 0x84, 0xBF, 0xBF, 0xAB, 0xE4, 0x50, 0x2E, 0xAF,
            0x24,
        ];

        for (index, byte) in expected.into_iter().enumerate() {
            let mut actual = 0;

            for bit in 0..8 {
                actual = actual << 1 | u8::from(message.bit(index * 8 + bit));
            }

            assert_eq!(actual, byte);
        }

        let segments = [Segment::numeric("0123456").unwrap()];
        let symbol =
            encode(&segments, RmqrVersion::R11x27, RmqrErrorCorrection::High, false, None).unwrap();
        let expected = [
            "#######.#.#.#.#.#.#.#.#.###",
            "#.....#..##.#....###.#..#.#",
            "#.###.#....#..####.#..#####",
            "#.###.#.####.##.####...##..",
            "#.###.#..#.###.######..#.##",
            "#.....#.###....#..#####..#.",
            "#######.#########...#.#####",
            "........#####.#.##.#..#...#",
            "####......#..#.#..#####.#.#",
            "#.#.#.#..##..#.#..###.#...#",
            "###.#.#.#.#.#.#.#.#.#.#####",
        ];

        for (actual, expected) in symbol.to_matrix().iter().zip(expected) {
            let actual: String =
                actual.iter().map(|&value| if value { '#' } else { '.' }).collect();
            assert_eq!(actual, expected);
        }
    }
}
