use alloc::{collections::VecDeque, vec, vec::Vec};

#[cfg(feature = "qr")]
use super::QrVersion;
#[cfg(feature = "kanji")]
use super::kanji_encoding;
use super::{EciAssignment, Mode, Segment, alphanumeric_value};
use crate::EncodeError;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Profile {
    mode_bits: u8,
    cci:       [u8; 4],
}

impl Profile {
    #[cfg(feature = "qr")]
    pub(crate) const fn qr(version: QrVersion) -> Self {
        let group = if version.value() <= 9 {
            0
        } else if version.value() <= 26 {
            1
        } else {
            2
        };

        Self {
            mode_bits: 4,
            cci:       [
                [10, 12, 14][group],
                [9, 11, 13][group],
                [8, 16, 16][group],
                [8, 10, 12][group],
            ],
        }
    }

    #[cfg(feature = "rmqr")]
    pub(crate) const fn rmqr(cci: [u8; 4]) -> Self {
        Self {
            mode_bits: 3,
            cci,
        }
    }

    #[inline]
    pub(crate) const fn cci_bits(self, mode: Mode) -> u8 {
        match mode {
            Mode::Numeric => self.cci[0],
            Mode::Alphanumeric => self.cci[1],
            Mode::Byte => self.cci[2],
            Mode::Kanji => self.cci[3],
            Mode::Eci => 0,
        }
    }

    #[inline]
    const fn eci_bits(self) -> usize {
        self.mode_bits as usize + 8
    }
}

#[derive(Clone, Copy)]
struct ByteStep {
    bits:     usize,
    segments: usize,
    previous: usize,
    mode:     Mode,
}

pub(crate) fn bytes(
    data: &[u8],
    profile: Profile,
    fnc1: bool,
) -> Result<Vec<Segment>, EncodeError> {
    // Each offset stores the shortest complete segmentation of the preceding bytes.
    let mut best = vec![None; data.len() + 1];
    best[0] = Some(ByteStep {
        bits: 0, segments: 0, previous: 0, mode: Mode::Byte
    });

    let byte_overhead = profile.mode_bits as usize + profile.cci_bits(Mode::Byte) as usize;
    let byte_window = max_count(Mode::Byte, profile);
    // A monotonic queue keeps the cheapest in-range byte-segment start, making each byte edge O(1).
    let mut byte_starts: VecDeque<(isize, usize, usize)> = VecDeque::new();

    for end in 0..=data.len() {
        if end >= 1 {
            while byte_starts.front().is_some_and(|&(.., start)| start + byte_window < end) {
                byte_starts.pop_front();
            }

            if let Some(&(.., start)) = byte_starts.front() {
                let prefix = best[start].expect("a queued byte start stays reachable");

                update_byte(
                    &mut best[end],
                    prefix,
                    start,
                    Mode::Byte,
                    byte_overhead + (end - start) * 8,
                );
            }
        }

        if end == data.len() {
            break;
        }

        let start = end;

        let Some(prefix) = best[start] else {
            continue;
        };

        // The ordered queue lets any later end reuse this start as a byte-segment beginning.
        let key = (prefix.bits as isize - 8 * start as isize, prefix.segments, start);

        while byte_starts.back().is_some_and(|&back| back >= key) {
            byte_starts.pop_back();
        }

        byte_starts.push_back(key);

        let mut cursor = start;

        while cursor < data.len()
            && cursor - start < max_count(Mode::Numeric, profile)
            && data[cursor].is_ascii_digit()
        {
            cursor += 1;
            update_byte(
                &mut best[cursor],
                prefix,
                start,
                Mode::Numeric,
                profile.mode_bits as usize
                    + profile.cci_bits(Mode::Numeric) as usize
                    + numeric_bits(cursor - start),
            );
        }

        // FNC1 encodes a literal percent as two alphanumeric characters.
        let mut encoded_count = 0;

        cursor = start;

        while cursor < data.len() {
            let byte = data[cursor];

            if alphanumeric_value(byte).is_none() && !(fnc1 && byte == 0x1D) {
                break;
            }

            encoded_count += usize::from(fnc1 && byte == b'%') + 1;

            if encoded_count > max_count(Mode::Alphanumeric, profile) {
                break;
            }

            cursor += 1;
            update_byte(
                &mut best[cursor],
                prefix,
                start,
                Mode::Alphanumeric,
                profile.mode_bits as usize
                    + profile.cci_bits(Mode::Alphanumeric) as usize
                    + alphanumeric_bits(encoded_count),
            );
        }
    }

    reconstruct_bytes(data, best, fnc1)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Interpretation {
    Latin1 = 0,
    Utf8   = 1,
}

#[derive(Clone, Copy)]
struct TextStep {
    bits:                    usize,
    switches:                usize,
    segments:                usize,
    previous_position:       usize,
    previous_interpretation: Interpretation,
    mode:                    Mode,
    interpretation:          Interpretation,
    switched:                bool,
}

#[derive(Clone, Copy)]
struct ByteEntry {
    // Candidate cost with the end-dependent term removed so entries compare independently of the end.
    cost:     isize,
    switches: usize,
    segments: usize,
    start:    usize,
    step:     TextStep,
    active:   Interpretation,
}

impl ByteEntry {
    #[inline]
    const fn key(self) -> (isize, usize, usize, usize) {
        (self.cost, self.switches, self.segments, self.start)
    }
}

pub(crate) fn text(
    text: &str,
    profile: Profile,
    fnc1: bool,
    force_initial_eci: bool,
) -> Result<Vec<Segment>, EncodeError> {
    let mut offsets: Vec<usize> = text.char_indices().map(|(offset, _)| offset).collect();

    offsets.push(text.len());

    let length = offsets.len() - 1;

    // Each position keeps the shortest path for both active byte interpretations.
    let mut best = vec![[None, None]; length + 1];

    best[0][Interpretation::Latin1 as usize] = Some(TextStep {
        bits:                    usize::from(force_initial_eci) * profile.eci_bits(),
        switches:                usize::from(force_initial_eci),
        segments:                usize::from(force_initial_eci),
        previous_position:       0,
        previous_interpretation: Interpretation::Latin1,
        mode:                    Mode::Byte,
        interpretation:          Interpretation::Latin1,
        switched:                false,
    });

    // A following Structured Append symbol must declare the interpretation that starts its data.
    if force_initial_eci {
        best[0][Interpretation::Utf8 as usize] = Some(TextStep {
            bits:                    profile.eci_bits(),
            switches:                1,
            segments:                1,
            previous_position:       0,
            previous_interpretation: Interpretation::Utf8,
            mode:                    Mode::Byte,
            interpretation:          Interpretation::Utf8,
            switched:                false,
        });
    }

    let eci_bits = profile.eci_bits();
    let byte_overhead = profile.mode_bits as usize + profile.cci_bits(Mode::Byte) as usize;
    let byte_window = max_count(Mode::Byte, profile);
    // Latin1 byte segments cannot span a character above U+00FF.
    let wide: Vec<bool> = text.chars().map(|character| u32::from(character) > 0xFF).collect();

    // A monotonic queue per target keeps the cheapest in-range byte-segment start, making byte edges O(1).
    let mut latin1_bytes: VecDeque<ByteEntry> = VecDeque::new();
    let mut utf8_bytes: VecDeque<ByteEntry> = VecDeque::new();

    for position in 0..=length {
        if position >= 1 {
            while utf8_bytes
                .front()
                .is_some_and(|entry| offsets[entry.start] + byte_window < offsets[position])
            {
                utf8_bytes.pop_front();
            }

            if let Some(entry) = utf8_bytes.front().copied() {
                let switched = entry.active != Interpretation::Utf8;

                update_text(
                    &mut best[position][Interpretation::Utf8 as usize],
                    entry.step,
                    entry.start,
                    entry.active,
                    Mode::Byte,
                    Interpretation::Utf8,
                    switched,
                    usize::from(switched) * eci_bits
                        + byte_overhead
                        + (offsets[position] - offsets[entry.start]) * 8,
                );
            }

            if wide[position - 1] {
                latin1_bytes.clear();
            } else {
                while latin1_bytes.front().is_some_and(|entry| entry.start + byte_window < position)
                {
                    latin1_bytes.pop_front();
                }

                if let Some(entry) = latin1_bytes.front().copied() {
                    let switched = entry.active != Interpretation::Latin1;

                    update_text(
                        &mut best[position][Interpretation::Latin1 as usize],
                        entry.step,
                        entry.start,
                        entry.active,
                        Mode::Byte,
                        Interpretation::Latin1,
                        switched,
                        usize::from(switched) * eci_bits
                            + byte_overhead
                            + (position - entry.start) * 8,
                    );
                }
            }
        }

        if position == length {
            break;
        }

        let start = position;

        for active in [Interpretation::Latin1, Interpretation::Utf8] {
            let Some(prefix) = best[start][active as usize] else {
                continue;
            };

            // Queue this start as a byte-segment source for the Latin1 target.
            {
                let switched = active != Interpretation::Latin1;
                let entry = ByteEntry {
                    cost: prefix.bits as isize + (usize::from(switched) * eci_bits) as isize
                        - 8 * start as isize,
                    switches: prefix.switches + usize::from(switched),
                    segments: prefix.segments + usize::from(switched),
                    start,
                    step: prefix,
                    active,
                };

                while latin1_bytes.back().is_some_and(|back| back.key() > entry.key()) {
                    latin1_bytes.pop_back();
                }

                latin1_bytes.push_back(entry);
            }

            // Queue this start as a byte-segment source for the Utf8 target.
            {
                let switched = active != Interpretation::Utf8;
                let entry = ByteEntry {
                    cost: prefix.bits as isize + (usize::from(switched) * eci_bits) as isize
                        - 8 * offsets[start] as isize,
                    switches: prefix.switches + usize::from(switched),
                    segments: prefix.segments + usize::from(switched),
                    start,
                    step: prefix,
                    active,
                };

                while utf8_bytes.back().is_some_and(|back| back.key() > entry.key()) {
                    utf8_bytes.pop_back();
                }

                utf8_bytes.push_back(entry);
            }

            let mut end = start;

            while end < length
                && end - start < max_count(Mode::Numeric, profile)
                && offsets[end + 1] == offsets[end] + 1
                && text.as_bytes()[offsets[end]].is_ascii_digit()
            {
                end += 1;

                update_text(
                    &mut best[end][active as usize],
                    prefix,
                    start,
                    active,
                    Mode::Numeric,
                    active,
                    false,
                    profile.mode_bits as usize
                        + profile.cci_bits(Mode::Numeric) as usize
                        + numeric_bits(end - start),
                );
            }

            let mut encoded_count = 0;

            end = start;

            while end < length {
                let bytes = &text.as_bytes()[offsets[end]..offsets[end + 1]];

                if bytes.len() != 1
                    || (alphanumeric_value(bytes[0]).is_none() && !(fnc1 && bytes[0] == 0x1D))
                {
                    break;
                }

                encoded_count += usize::from(fnc1 && bytes[0] == b'%') + 1;

                if encoded_count > max_count(Mode::Alphanumeric, profile) {
                    break;
                }

                end += 1;

                update_text(
                    &mut best[end][active as usize],
                    prefix,
                    start,
                    active,
                    Mode::Alphanumeric,
                    active,
                    false,
                    profile.mode_bits as usize
                        + profile.cci_bits(Mode::Alphanumeric) as usize
                        + alphanumeric_bits(encoded_count),
                );
            }

            #[cfg(feature = "kanji")]
            {
                end = start;

                while end < length && end - start < max_count(Mode::Kanji, profile) {
                    let character = text[offsets[end]..offsets[end + 1]]
                        .chars()
                        .next()
                        .expect("the range contains one character");

                    if kanji_encoding(character).is_none() {
                        break;
                    }

                    end += 1;

                    update_text(
                        &mut best[end][active as usize],
                        prefix,
                        start,
                        active,
                        Mode::Kanji,
                        active,
                        false,
                        profile.mode_bits as usize
                            + profile.cci_bits(Mode::Kanji) as usize
                            + (end - start) * 13,
                    );
                }
            }
        }
    }

    let final_interpretation = [Interpretation::Latin1, Interpretation::Utf8]
        .into_iter()
        .filter_map(|interpretation| {
            best[length][interpretation as usize].map(|step| (interpretation, step))
        })
        .min_by_key(|(_, step)| (step.bits, step.switches, step.segments))
        .map(|(interpretation, _)| interpretation)
        .ok_or(EncodeError::DataTooLong {
            required_bits: usize::MAX, capacity_bits: 0
        })?;

    // Backtracking also recovers the interpretation selected before the first segment.
    let mut edges = Vec::new();
    let mut position = length;
    let mut interpretation = final_interpretation;

    while position != 0 {
        let step = best[position][interpretation as usize].expect("the final state is reachable");
        edges.push((step.previous_position, position, step));
        position = step.previous_position;
        interpretation = step.previous_interpretation;
    }

    edges.reverse();

    let mut result = Vec::new();

    if force_initial_eci {
        result.push(Segment::eci(match interpretation {
            Interpretation::Latin1 => EciAssignment::ISO_8859_1,
            Interpretation::Utf8 => EciAssignment::UTF_8,
        }));
    }

    for (start, end, step) in edges {
        if step.switched {
            result.push(Segment::eci(match step.interpretation {
                Interpretation::Latin1 => EciAssignment::ISO_8859_1,
                Interpretation::Utf8 => EciAssignment::UTF_8,
            }));
        }

        let slice = &text[offsets[start]..offsets[end]];

        result.push(match step.mode {
            Mode::Numeric => Segment::numeric(slice)?,
            Mode::Alphanumeric if fnc1 => Segment::fnc1_alphanumeric(slice.as_bytes())?,
            Mode::Alphanumeric => Segment::alphanumeric(slice)?,
            Mode::Byte if step.interpretation == Interpretation::Latin1 => {
                let bytes: Vec<u8> = slice.chars().map(|character| character as u8).collect();
                Segment::bytes(&bytes)
            },
            Mode::Byte => Segment::bytes(slice.as_bytes()),
            #[cfg(feature = "kanji")]
            Mode::Kanji => Segment::kanji(slice)?,
            #[cfg(not(feature = "kanji"))]
            Mode::Kanji => unreachable!(),
            Mode::Eci => unreachable!(),
        });
    }
    Ok(result)
}

fn reconstruct_bytes(
    data: &[u8],
    best: Vec<Option<ByteStep>>,
    fnc1: bool,
) -> Result<Vec<Segment>, EncodeError> {
    let mut position = data.len();
    let mut ranges = Vec::new();

    while position != 0 {
        let step = best[position]
            .ok_or(EncodeError::DataTooLong {
                required_bits: usize::MAX, capacity_bits: 0
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
            Mode::Alphanumeric if fnc1 => Segment::fnc1_alphanumeric(&data[start..end]),
            Mode::Alphanumeric => Segment::alphanumeric(
                core::str::from_utf8(&data[start..end]).expect("alphanumeric data is UTF-8"),
            ),
            Mode::Byte => Ok(Segment::bytes(&data[start..end])),
            Mode::Kanji | Mode::Eci => unreachable!(),
        })
        .collect()
}

fn update_byte(
    slot: &mut Option<ByteStep>,
    prefix: ByteStep,
    previous: usize,
    mode: Mode,
    segment_bits: usize,
) {
    let candidate = ByteStep {
        bits: prefix.bits + segment_bits,
        segments: prefix.segments + 1,
        previous,
        mode,
    };

    if slot.is_none_or(|current| {
        (candidate.bits, candidate.segments, mode_rank(candidate.mode), candidate.previous)
            < (current.bits, current.segments, mode_rank(current.mode), current.previous)
    }) {
        *slot = Some(candidate);
    }
}

#[allow(clippy::too_many_arguments)]
fn update_text(
    slot: &mut Option<TextStep>,
    prefix: TextStep,
    previous_position: usize,
    previous_interpretation: Interpretation,
    mode: Mode,
    interpretation: Interpretation,
    switched: bool,
    edge_bits: usize,
) {
    let candidate = TextStep {
        bits: prefix.bits + edge_bits,
        switches: prefix.switches + usize::from(switched),
        segments: prefix.segments + 1 + usize::from(switched),
        previous_position,
        previous_interpretation,
        mode,
        interpretation,
        switched,
    };

    if slot.is_none_or(|current| {
        (
            candidate.bits,
            candidate.switches,
            candidate.segments,
            mode_rank(candidate.mode),
            candidate.previous_position,
        ) < (
            current.bits,
            current.switches,
            current.segments,
            mode_rank(current.mode),
            current.previous_position,
        )
    }) {
        *slot = Some(candidate);
    }
}

#[inline]
const fn max_count(mode: Mode, profile: Profile) -> usize {
    (1usize << profile.cci_bits(mode)) - 1
}

#[inline]
const fn numeric_bits(count: usize) -> usize {
    count / 3 * 10
        + match count % 3 {
            1 => 4,
            2 => 7,
            _ => 0,
        }
}

#[inline]
const fn alphanumeric_bits(count: usize) -> usize {
    count / 2 * 11 + count % 2 * 6
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
