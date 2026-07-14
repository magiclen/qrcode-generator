use alloc::{vec, vec::Vec};

#[cfg(feature = "kanji")]
use super::kanji_encoding;
use super::{
    MicroErrorCorrection, MicroMask, MicroVersion, Mode, Segment, Symbol, SymbolVersion,
    alphanumeric_value, bits::BitBuffer, reed_solomon,
};
use crate::EncodeError;

pub(crate) fn optimize(data: &[u8], version: MicroVersion) -> Result<Vec<Segment>, EncodeError> {
    #[derive(Clone, Copy)]
    struct Step {
        bits:     usize,
        segments: usize,
        previous: usize,
        mode:     Mode,
    }

    // Each offset stores the shortest segmentation using only modes available to this Micro version.
    let mut best = vec![None; data.len() + 1];

    best[0] = Some(Step {
        bits: 0, segments: 0, previous: 0, mode: Mode::Numeric
    });

    for start in 0..data.len() {
        let Some(prefix) = best[start] else {
            continue;
        };

        for &mode in available_modes(version) {
            let Some((indicator_bits, cci_bits)) = mode_parameters(version, mode) else {
                continue;
            };

            let maximum = ((1usize << cci_bits) - 1).min(data.len() - start);

            for count in 1..=maximum {
                let slice = &data[start..start + count];
                let payload_bits = match mode {
                    Mode::Numeric if slice.iter().all(u8::is_ascii_digit) => {
                        count / 3 * 10 + [0, 4, 7][count % 3]
                    },
                    Mode::Alphanumeric
                        if slice.iter().all(|byte| alphanumeric_value(*byte).is_some()) =>
                    {
                        count / 2 * 11 + count % 2 * 6
                    },
                    Mode::Byte => count * 8,
                    _ => break,
                };

                let candidate = Step {
                    bits: prefix.bits + indicator_bits as usize + cci_bits as usize + payload_bits,
                    segments: prefix.segments + 1,
                    previous: start,
                    mode,
                };

                let slot = &mut best[start + count];

                if slot.is_none_or(|current| {
                    (
                        candidate.bits,
                        candidate.segments,
                        mode_rank(candidate.mode),
                        candidate.previous,
                    ) < (current.bits, current.segments, mode_rank(current.mode), current.previous)
                }) {
                    *slot = Some(candidate);
                }
            }
        }
    }

    let mut position = data.len();
    let mut ranges = Vec::new();

    while position != 0 {
        let step = best[position].ok_or(EncodeError::UnsupportedMode {
            mode:   "input data",
            family: "the selected Micro QR version",
        })?;
        ranges.push((step.previous, position, step.mode));
        position = step.previous;
    }

    ranges.reverse();

    ranges
        .into_iter()
        .map(|(start, end, mode)| match mode {
            Mode::Numeric => Segment::numeric(
                core::str::from_utf8(&data[start..end]).expect("numeric data is UTF-8"),
            ),
            Mode::Alphanumeric => Segment::alphanumeric(
                core::str::from_utf8(&data[start..end]).expect("alphanumeric data is UTF-8"),
            ),
            Mode::Byte => Ok(Segment::bytes(&data[start..end])),
            Mode::Kanji | Mode::Eci => unreachable!(),
        })
        .collect()
}

pub(crate) fn optimize_text(
    text: &str,
    version: MicroVersion,
) -> Result<Vec<Segment>, EncodeError> {
    #[derive(Clone, Copy)]
    struct Step {
        bits:     usize,
        segments: usize,
        previous: usize,
        mode:     Mode,
    }

    let mut offsets: Vec<_> = text.char_indices().map(|(offset, _)| offset).collect();

    offsets.push(text.len());

    let length = offsets.len() - 1;

    #[cfg(feature = "kanji")]
    let kanji: Vec<bool> =
        text.chars().map(|character| kanji_encoding(character).is_some()).collect();

    // Scalar boundaries keep multibyte UTF-8 characters intact while evaluating Kanji mode.
    let mut best = vec![None; length + 1];

    best[0] = Some(Step {
        bits: 0, segments: 0, previous: 0, mode: Mode::Numeric
    });

    for start in 0..length {
        let Some(prefix) = best[start] else {
            continue;
        };

        for mode in [Mode::Numeric, Mode::Alphanumeric, Mode::Byte, Mode::Kanji] {
            let Some((indicator, cci)) = mode_parameters(version, mode) else {
                continue;
            };

            let maximum = (1usize << cci) - 1;

            for end in start + 1..=length.min(start + maximum) {
                let slice = &text[offsets[start]..offsets[end]];

                let payload = match mode {
                    Mode::Numeric if slice.bytes().all(|byte| byte.is_ascii_digit()) => {
                        let count = end - start;
                        count / 3 * 10 + [0, 4, 7][count % 3]
                    },
                    Mode::Alphanumeric
                        if slice.is_ascii()
                            && slice.bytes().all(|byte| alphanumeric_value(byte).is_some()) =>
                    {
                        let count = end - start;
                        count / 2 * 11 + count % 2 * 6
                    },
                    Mode::Byte if slice.chars().all(|character| u32::from(character) <= 0xFF) => {
                        (end - start) * 8
                    },
                    #[cfg(feature = "kanji")]
                    Mode::Kanji if kanji[end - 1] => (end - start) * 13,
                    _ => break,
                };

                let candidate = Step {
                    bits: prefix.bits + indicator as usize + cci as usize + payload,
                    segments: prefix.segments + 1,
                    previous: start,
                    mode,
                };

                let slot = &mut best[end];

                if slot.is_none_or(|current| {
                    (
                        candidate.bits,
                        candidate.segments,
                        mode_rank(candidate.mode),
                        candidate.previous,
                    ) < (current.bits, current.segments, mode_rank(current.mode), current.previous)
                }) {
                    *slot = Some(candidate);
                }
            }
        }
    }

    if let Some(position) = best.iter().position(Option::is_none) {
        return Err(EncodeError::TextNotRepresentable {
            byte_offset: offsets[position - 1],
            family:      "Micro QR Code",
        });
    }

    let mut position = length;
    let mut ranges = Vec::new();

    while position != 0 {
        let step = best[position].expect("every text position is reachable");
        ranges.push((step.previous, position, step.mode));
        position = step.previous;
    }

    ranges.reverse();

    ranges
        .into_iter()
        .map(|(start, end, mode)| {
            let slice = &text[offsets[start]..offsets[end]];
            match mode {
                Mode::Numeric => Segment::numeric(slice),
                Mode::Alphanumeric => Segment::alphanumeric(slice),
                Mode::Byte => Ok(Segment::bytes(
                    slice.chars().map(|character| character as u8).collect::<Vec<_>>(),
                )),
                #[cfg(feature = "kanji")]
                Mode::Kanji => Segment::kanji(slice),
                #[cfg(not(feature = "kanji"))]
                Mode::Kanji => unreachable!(),
                Mode::Eci => unreachable!(),
            }
        })
        .collect()
}

pub(crate) fn encode(
    segments: &[Segment],
    version: MicroVersion,
    mut error_correction: MicroErrorCorrection,
    requested_mask: Option<MicroMask>,
    boost_error_correction: bool,
) -> Result<Symbol, EncodeError> {
    let mut capacity_info =
        capacity(version, error_correction).ok_or(EncodeError::UnsupportedErrorCorrection {
            version:          SymbolVersion::Micro(version),
            error_correction: error_correction.into(),
        })?;
    let used_bits = total_bits(segments, version)?;

    if used_bits > capacity_info.data_bits {
        return Err(EncodeError::DataTooLong {
            required_bits: used_bits,
            capacity_bits: capacity_info.data_bits,
        });
    }

    if boost_error_correction {
        for candidate in [MicroErrorCorrection::Medium, MicroErrorCorrection::Quartile] {
            if candidate > error_correction
                && let Some(candidate_capacity) = capacity(version, candidate)
                && used_bits <= candidate_capacity.data_bits
            {
                error_correction = candidate;
                capacity_info = candidate_capacity;
            }
        }
    }

    let mut data_bits = BitBuffer::with_capacity(capacity_info.data_bits);

    for segment in segments {
        let (indicator_bits, cci_bits) =
            mode_parameters(version, segment.mode).ok_or(EncodeError::UnsupportedMode {
                mode:   mode_name(segment.mode),
                family: "the selected Micro QR version",
            })?;

        if indicator_bits != 0 {
            data_bits.append(mode_indicator(version, segment.mode), indicator_bits);
        }

        data_bits.append(segment.character_count as u32, cci_bits);
        data_bits.extend(&segment.bits);
    }

    let terminator = match version {
        MicroVersion::M1 => 3,
        MicroVersion::M2 => 5,
        MicroVersion::M3 => 7,
        MicroVersion::M4 => 9,
    };

    data_bits.append(0, (capacity_info.data_bits - data_bits.len()).min(terminator) as u8);

    while data_bits.len() < capacity_info.data_bits && data_bits.len() & 7 != 0 {
        data_bits.push(false);
    }

    let mut pad = true;

    while data_bits.len() + 8 <= capacity_info.data_bits {
        data_bits.append(if pad { 0xEC } else { 0x11 }, 8);
        pad = !pad;
    }

    while data_bits.len() < capacity_info.data_bits {
        data_bits.push(false);
    }

    // M1 and M3 pad the final four data bits only while calculating the Reed-Solomon remainder.
    let data_bytes = data_bits.clone().into_padded_bytes();
    let ecc =
        reed_solomon::remainder(&data_bytes, &reed_solomon::divisor(capacity_info.ecc_codewords));
    let mut final_bits = data_bits;

    for byte in ecc {
        final_bits.append(u32::from(byte), 8);
    }

    debug_assert_eq!(
        final_bits.len(),
        version.size() * version.size() - function_module_count(version) - 15
    );

    let mut matrix = Matrix::new(version, error_correction);

    matrix.draw_data(&final_bits);

    let mask = if let Some(mask) = requested_mask {
        mask.value()
    } else {
        // Micro QR selects the candidate with the highest dark-edge score.
        let mut best_mask = 0;
        let mut best_score = -1;

        for candidate in 0..4 {
            matrix.apply_mask(candidate);
            matrix.draw_format(candidate);

            let score = matrix.score();

            if score > best_score {
                best_mask = candidate;
                best_score = score;
            }

            matrix.apply_mask(candidate);
        }

        best_mask
    };

    matrix.apply_mask(mask);
    matrix.draw_format(mask);

    Ok(Symbol {
        version: SymbolVersion::Micro(version),
        error_correction: error_correction.into(),
        mask,
        modules: matrix.modules,
        #[cfg(feature = "qr")]
        structured_append: None,
    })
}

fn total_bits(segments: &[Segment], version: MicroVersion) -> Result<usize, EncodeError> {
    let mut result = 0usize;

    for segment in segments {
        let (indicator, cci) =
            mode_parameters(version, segment.mode).ok_or(EncodeError::UnsupportedMode {
                mode:   mode_name(segment.mode),
                family: "the selected Micro QR version",
            })?;

        if segment.character_count >= 1usize << cci {
            return Err(EncodeError::DataTooLong {
                required_bits: usize::MAX, capacity_bits: 0
            });
        }

        result = result.checked_add(indicator as usize + cci as usize + segment.bits.len()).ok_or(
            EncodeError::DataTooLong {
                required_bits: usize::MAX, capacity_bits: 0
            },
        )?;
    }
    Ok(result)
}

#[inline]
const fn available_modes(version: MicroVersion) -> &'static [Mode] {
    match version {
        MicroVersion::M1 => &[Mode::Numeric],
        MicroVersion::M2 => &[Mode::Numeric, Mode::Alphanumeric],
        MicroVersion::M3 | MicroVersion::M4 => &[Mode::Numeric, Mode::Alphanumeric, Mode::Byte],
    }
}

#[inline]
const fn mode_parameters(version: MicroVersion, mode: Mode) -> Option<(u8, u8)> {
    let indicator = match version {
        MicroVersion::M1 => 0,
        MicroVersion::M2 => 1,
        MicroVersion::M3 => 2,
        MicroVersion::M4 => 3,
    };

    let cci = match (version, mode) {
        (MicroVersion::M1, Mode::Numeric) => 3,
        (MicroVersion::M2, Mode::Numeric) => 4,
        (MicroVersion::M2, Mode::Alphanumeric) => 3,
        (MicroVersion::M3, Mode::Numeric) => 5,
        (MicroVersion::M3, Mode::Alphanumeric | Mode::Byte) => 4,
        (MicroVersion::M3, Mode::Kanji) => 3,
        (MicroVersion::M4, Mode::Numeric) => 6,
        (MicroVersion::M4, Mode::Alphanumeric | Mode::Byte) => 5,
        (MicroVersion::M4, Mode::Kanji) => 4,
        _ => return None,
    };
    Some((indicator, cci))
}

#[inline]
const fn mode_indicator(version: MicroVersion, mode: Mode) -> u32 {
    match (version, mode) {
        (MicroVersion::M1, Mode::Numeric)
        | (MicroVersion::M2, Mode::Numeric)
        | (MicroVersion::M3, Mode::Numeric)
        | (MicroVersion::M4, Mode::Numeric) => 0,
        (MicroVersion::M2, Mode::Alphanumeric)
        | (MicroVersion::M3, Mode::Alphanumeric)
        | (MicroVersion::M4, Mode::Alphanumeric) => 1,
        (MicroVersion::M3, Mode::Byte) | (MicroVersion::M4, Mode::Byte) => 2,
        (MicroVersion::M3, Mode::Kanji) | (MicroVersion::M4, Mode::Kanji) => 3,
        _ => unreachable!(),
    }
}

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
const fn mode_name(mode: Mode) -> &'static str {
    match mode {
        Mode::Numeric => "numeric",
        Mode::Alphanumeric => "alphanumeric",
        Mode::Byte => "byte",
        Mode::Kanji => "Kanji",
        Mode::Eci => "ECI",
    }
}

#[derive(Clone, Copy)]
struct Capacity {
    data_bits:     usize,
    ecc_codewords: usize,
}

const fn capacity(
    version: MicroVersion,
    error_correction: MicroErrorCorrection,
) -> Option<Capacity> {
    let values = match (version, error_correction) {
        (MicroVersion::M1, MicroErrorCorrection::DetectionOnly) => (20, 2),
        (MicroVersion::M2, MicroErrorCorrection::Low) => (40, 5),
        (MicroVersion::M2, MicroErrorCorrection::Medium) => (32, 6),
        (MicroVersion::M3, MicroErrorCorrection::Low) => (84, 6),
        (MicroVersion::M3, MicroErrorCorrection::Medium) => (68, 8),
        (MicroVersion::M4, MicroErrorCorrection::Low) => (128, 8),
        (MicroVersion::M4, MicroErrorCorrection::Medium) => (112, 10),
        (MicroVersion::M4, MicroErrorCorrection::Quartile) => (80, 14),
        _ => return None,
    };

    Some(Capacity {
        data_bits: values.0, ecc_codewords: values.1
    })
}

pub(crate) fn input_capacity_upper_bound(
    version: MicroVersion,
    error_correction: MicroErrorCorrection,
) -> Option<(usize, usize)> {
    let capacity = capacity(version, error_correction)?;
    let (indicator_bits, cci_bits) = mode_parameters(version, Mode::Numeric)?;
    let overhead_bits = indicator_bits as usize + cci_bits as usize;

    Some((super::numeric_character_capacity(capacity.data_bits, overhead_bits), capacity.data_bits))
}

#[inline]
const fn function_module_count(version: MicroVersion) -> usize {
    match version {
        MicroVersion::M1 => 70,
        MicroVersion::M2 => 74,
        MicroVersion::M3 => 78,
        MicroVersion::M4 => 82,
    }
}

struct Matrix {
    version:          MicroVersion,
    error_correction: MicroErrorCorrection,
    size:             usize,
    modules:          Vec<bool>,
    function:         Vec<bool>,
}

impl Matrix {
    fn new(version: MicroVersion, error_correction: MicroErrorCorrection) -> Self {
        let size = version.size();
        let mut result = Self {
            version,
            error_correction,
            size,
            modules: vec![false; size * size],
            function: vec![false; size * size],
        };

        for y in 0..7 {
            for x in 0..7 {
                let distance = (x as isize - 3).abs().max((y as isize - 3).abs());

                result.set_function(x, y, distance == 3 || distance <= 1);
            }
        }

        for coordinate in 0..=7 {
            result.set_function(coordinate, 7, false);
            result.set_function(7, coordinate, false);
        }

        for coordinate in 8..size {
            result.set_function(coordinate, 0, (coordinate - 8) % 2 == 0);
            result.set_function(0, coordinate, (coordinate - 8) % 2 == 0);
        }

        for offset in 0..8 {
            result.set_function(8, 1 + offset, false);
        }

        for offset in 0..7 {
            result.set_function(7 - offset, 8, false);
        }

        result
    }

    fn draw_data(&mut self, bits: &BitBuffer) {
        // Micro QR uses alternating two-column stripes without the Model 2 timing-column skip.
        let mut index = 0;
        let mut right = self.size - 1;
        let mut upward = true;

        while right >= 1 {
            for vertical in 0..self.size {
                let y = if upward { self.size - 1 - vertical } else { vertical };

                for offset in 0..2 {
                    let x = right - offset;
                    let module = y * self.size + x;
                    if !self.function[module] && index < bits.len() {
                        self.modules[module] = bits.bit(index);
                        index += 1;
                    }
                }
            }

            if right < 2 {
                break;
            }

            right -= 2;
            upward = !upward;
        }

        debug_assert_eq!(index, bits.len());
    }

    fn apply_mask(&mut self, mask: u8) {
        for y in 0..self.size {
            for x in 0..self.size {
                let invert = match mask {
                    0 => y % 2 == 0,
                    1 => (y / 2 + x / 3) % 2 == 0,
                    2 => (y * x % 2 + y * x % 3) % 2 == 0,
                    3 => ((y + x) % 2 + y * x % 3) % 2 == 0,
                    _ => unreachable!(),
                };

                let index = y * self.size + x;

                if invert && !self.function[index] {
                    self.modules[index] = !self.modules[index];
                }
            }
        }
    }

    fn draw_format(&mut self, mask: u8) {
        let data = u32::from(symbol_number(self.version, self.error_correction) << 2 | mask);
        let mut remainder = data;

        for _ in 0..10 {
            remainder = (remainder << 1) ^ ((remainder >> 9) * 0x537);
        }

        let bits = (data << 10 | remainder) ^ 0x4445;

        for offset in 0..8 {
            self.set_function(8, 1 + offset, bits >> offset & 1 != 0);
        }

        for offset in 0..7 {
            self.set_function(7 - offset, 8, bits >> (8 + offset) & 1 != 0);
        }
    }

    fn score(&self) -> i32 {
        // The smaller edge count is the high-order part so balanced dark edges score better.
        let right =
            (1..self.size).filter(|&y| self.modules[y * self.size + self.size - 1]).count() as i32;

        let bottom = (1..self.size)
            .filter(|&x| self.modules[(self.size - 1) * self.size + x])
            .count() as i32;

        16 * right.min(bottom) + right.max(bottom)
    }

    #[inline]
    fn set_function(&mut self, x: usize, y: usize, value: bool) {
        let index = y * self.size + x;

        self.modules[index] = value;
        self.function[index] = true;
    }
}

#[inline]
const fn symbol_number(version: MicroVersion, error_correction: MicroErrorCorrection) -> u8 {
    match (version, error_correction) {
        (MicroVersion::M1, MicroErrorCorrection::DetectionOnly) => 0,
        (MicroVersion::M2, MicroErrorCorrection::Low) => 1,
        (MicroVersion::M2, MicroErrorCorrection::Medium) => 2,
        (MicroVersion::M3, MicroErrorCorrection::Low) => 3,
        (MicroVersion::M3, MicroErrorCorrection::Medium) => 4,
        (MicroVersion::M4, MicroErrorCorrection::Low) => 5,
        (MicroVersion::M4, MicroErrorCorrection::Medium) => 6,
        (MicroVersion::M4, MicroErrorCorrection::Quartile) => 7,
        _ => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    use alloc::string::String;

    use super::*;

    #[test]
    fn unrepresentable_text_reports_the_first_byte_offset() {
        for (text, byte_offset) in [("😀A", 0), ("A😀B", 1)] {
            assert!(matches!(
                optimize_text(text, MicroVersion::M4),
                Err(EncodeError::TextNotRepresentable {
                    byte_offset: actual,
                    ..
                }) if actual == byte_offset
            ));
        }
    }

    #[test]
    fn every_version_accepts_its_numeric_capacity_and_rejects_one_more() {
        for (version, error_correction, character_count) in [
            (MicroVersion::M1, MicroErrorCorrection::DetectionOnly, 5),
            (MicroVersion::M2, MicroErrorCorrection::Low, 10),
            (MicroVersion::M2, MicroErrorCorrection::Medium, 8),
            (MicroVersion::M3, MicroErrorCorrection::Low, 23),
            (MicroVersion::M3, MicroErrorCorrection::Medium, 18),
            (MicroVersion::M4, MicroErrorCorrection::Low, 35),
            (MicroVersion::M4, MicroErrorCorrection::Medium, 30),
            (MicroVersion::M4, MicroErrorCorrection::Quartile, 21),
        ] {
            let segment = Segment::numeric("1".repeat(character_count)).unwrap();
            assert!(
                encode(&[segment], version, error_correction, Some(MicroMask(0)), false,).is_ok()
            );

            let too_long = Segment::numeric("1".repeat(character_count + 1)).unwrap();
            assert!(matches!(
                encode(&[too_long], version, error_correction, Some(MicroMask(0)), false,),
                Err(EncodeError::DataTooLong { .. })
            ));
        }
    }

    #[test]
    fn annex_i_m2_l_matrix() {
        let segments = optimize(b"01234567", MicroVersion::M2).unwrap();
        let symbol = encode(
            &segments,
            MicroVersion::M2,
            MicroErrorCorrection::Low,
            Some(MicroMask::new(1).unwrap()),
            false,
        )
        .unwrap();
        let expected = [
            "1111111010101",
            "1000001011101",
            "1011101001101",
            "1011101001111",
            "1011101011100",
            "1000001010001",
            "1111111001111",
            "0000000001100",
            "1101000010001",
            "0110101010101",
            "1110011111110",
            "0001010000110",
            "1110100110111",
        ];
        for (row, expected) in symbol.modules.chunks(13).zip(expected) {
            assert_eq!(
                row.iter().map(|value| if *value { '1' } else { '0' }).collect::<String>(),
                expected
            );
        }
    }
}
