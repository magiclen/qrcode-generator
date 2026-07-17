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
    #[inline]
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
    #[inline]
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
    const fn overhead_bits(self, mode: Mode) -> usize {
        self.mode_bits as usize + self.cci_bits(mode) as usize
    }

    #[inline]
    const fn eci_bits(self) -> usize {
        self.mode_bits as usize + 8
    }
}

// The character interpretation in force, following the strict ECI semantics of ISO/IEC 18004.
// `Default` means no ECI header has been emitted yet, so byte data reads as ISO-8859-1 and Kanji mode reads as Shift JIS.
// The other states are entered by emitting the matching character set ECI header.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Interpretation {
    Default  = 0,
    Latin1   = 1,
    Utf8     = 2,
    #[cfg(feature = "kanji")]
    ShiftJis = 3,
}

impl Interpretation {
    #[cfg(feature = "kanji")]
    const ALL: [Self; 4] = [Self::Default, Self::Latin1, Self::Utf8, Self::ShiftJis];
    #[cfg(not(feature = "kanji"))]
    const ALL: [Self; 3] = [Self::Default, Self::Latin1, Self::Utf8];
    const COUNT: usize = Self::ALL.len();

    #[inline]
    const fn index(self) -> usize {
        self as usize
    }

    // Returns the ECI assignment that this interpretation declares, or `None` for the default one.
    #[inline]
    const fn eci(self) -> Option<EciAssignment> {
        match self {
            Self::Default => None,
            Self::Latin1 => Some(EciAssignment::ISO_8859_1),
            Self::Utf8 => Some(EciAssignment::UTF_8),
            #[cfg(feature = "kanji")]
            Self::ShiftJis => Some(EciAssignment::SHIFT_JIS),
        }
    }
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

// One queued segment start with the end-dependent cost term removed, so entries compare independently of the end.
#[derive(Clone, Copy)]
struct QueueEntry {
    cost:       isize,
    switches:   usize,
    segments:   usize,
    start:      usize,
    // The start position measured in the coordinate of its queue: characters, UTF-8 bytes, Shift JIS bytes or encoded counts.
    coordinate: usize,
    step:       TextStep,
    active:     Interpretation,
}

impl QueueEntry {
    #[inline]
    const fn key(self) -> (isize, usize, usize, usize) {
        (self.cost, self.switches, self.segments, self.start)
    }
}

#[inline]
fn push_entry(queue: &mut VecDeque<QueueEntry>, entry: QueueEntry) {
    while queue.back().is_some_and(|back| back.key() > entry.key()) {
        queue.pop_back();
    }

    queue.push_back(entry);
}

// Per-character tables shared by the dynamic program and the reconstruction.
struct TextTables {
    offsets:      Vec<usize>,
    wide:         Vec<bool>,
    digit:        Vec<bool>,
    alnum_ok:     Vec<bool>,
    // Prefix sums of encoded alphanumeric characters; FNC1 doubles a literal percent.
    alnum_prefix: Vec<usize>,
    #[cfg(feature = "kanji")]
    kanji_ok:     Vec<bool>,
    #[cfg(feature = "kanji")]
    sjis_ok:      Vec<bool>,
    // Prefix sums of Shift JIS byte lengths; positions after an unencodable character keep the previous sum.
    #[cfg(feature = "kanji")]
    sjis_prefix:  Vec<usize>,
}

impl TextTables {
    fn new(text: &str, fnc1: bool) -> Self {
        let mut offsets: Vec<usize> = text.char_indices().map(|(offset, _)| offset).collect();

        offsets.push(text.len());

        let length = offsets.len() - 1;
        let mut wide = Vec::with_capacity(length);
        let mut digit = Vec::with_capacity(length);
        let mut alnum_ok = Vec::with_capacity(length);
        let mut alnum_prefix = Vec::with_capacity(length + 1);
        #[cfg(feature = "kanji")]
        let mut kanji_ok = Vec::with_capacity(length);
        #[cfg(feature = "kanji")]
        let mut sjis_ok = Vec::with_capacity(length);
        #[cfg(feature = "kanji")]
        let mut sjis_prefix = Vec::with_capacity(length + 1);

        alnum_prefix.push(0);
        #[cfg(feature = "kanji")]
        sjis_prefix.push(0);

        for character in text.chars() {
            wide.push(u32::from(character) > 0xFF);
            digit.push(character.is_ascii_digit());

            let byte = character as u32;
            let eligible = character.is_ascii()
                && (alphanumeric_value(byte as u8).is_some() || (fnc1 && byte == 0x1D));

            alnum_ok.push(eligible);

            let encoded = usize::from(eligible) * (1 + usize::from(fnc1 && character == '%'))
                + usize::from(!eligible);

            alnum_prefix
                .push(alnum_prefix.last().copied().expect("the prefix starts at zero") + encoded);

            #[cfg(feature = "kanji")]
            {
                kanji_ok.push(kanji_encoding(character).is_some());

                let previous = sjis_prefix.last().copied().expect("the prefix starts at zero");

                match sjis_byte_length(character) {
                    Some(bytes) => {
                        sjis_ok.push(true);
                        sjis_prefix.push(previous + bytes);
                    },
                    None => {
                        sjis_ok.push(false);
                        sjis_prefix.push(previous);
                    },
                }
            }
        }

        Self {
            offsets,
            wide,
            digit,
            alnum_ok,
            alnum_prefix,
            #[cfg(feature = "kanji")]
            kanji_ok,
            #[cfg(feature = "kanji")]
            sjis_ok,
            #[cfg(feature = "kanji")]
            sjis_prefix,
        }
    }
}

// Returns the Shift JIS byte length of a character usable in a byte segment under ECI 20.
// Backslash, tilde, yen and overline are rejected because WHATWG and JIS X 0201 decode bytes 5C and 7E differently.
#[cfg(feature = "kanji")]
fn sjis_byte_length(character: char) -> Option<usize> {
    match character {
        '\\' | '~' | '¥' | '\u{203E}' => None,
        _ if character.is_ascii() => Some(1),
        '\u{FF61}'..='\u{FF9F}' => Some(1),
        _ => {
            let mut utf8 = [0; 4];
            let text = character.encode_utf8(&mut utf8);
            let (encoded, _, had_errors) = encoding_rs::SHIFT_JIS.encode(text);

            (!had_errors).then(|| encoded.len())
        },
    }
}

pub(crate) fn text(
    text: &str,
    profile: Profile,
    fnc1: bool,
    force_initial_eci: bool,
) -> Result<Vec<Segment>, EncodeError> {
    let tables = TextTables::new(text, fnc1);
    let length = tables.offsets.len() - 1;

    // Each position keeps the shortest path for every interpretation that can be in force there.
    let mut best = vec![[None; Interpretation::COUNT]; length + 1];

    // A following Structured Append symbol must declare the interpretation that starts its data.
    if force_initial_eci {
        for interpretation in Interpretation::ALL {
            if interpretation == Interpretation::Default {
                continue;
            }

            best[0][interpretation.index()] = Some(TextStep {
                bits: profile.eci_bits(),
                switches: 1,
                segments: 1,
                previous_position: 0,
                previous_interpretation: interpretation,
                mode: Mode::Byte,
                interpretation,
                switched: false,
            });
        }
    } else {
        best[0][Interpretation::Default.index()] = Some(TextStep {
            bits:                    0,
            switches:                0,
            segments:                0,
            previous_position:       0,
            previous_interpretation: Interpretation::Default,
            mode:                    Mode::Byte,
            interpretation:          Interpretation::Default,
            switched:                false,
        });
    }

    let eci_bits = profile.eci_bits();
    let numeric_overhead = profile.overhead_bits(Mode::Numeric);
    let alnum_overhead = profile.overhead_bits(Mode::Alphanumeric);
    let byte_overhead = profile.overhead_bits(Mode::Byte);
    #[cfg(feature = "kanji")]
    let kanji_overhead = profile.overhead_bits(Mode::Kanji);

    let numeric_window = max_count(Mode::Numeric, profile);
    let alnum_window = max_count(Mode::Alphanumeric, profile);
    let byte_window = max_count(Mode::Byte, profile);
    #[cfg(feature = "kanji")]
    let kanji_window = max_count(Mode::Kanji, profile);

    // One monotonic queue per target keeps the cheapest in-range segment start, making every edge O(1) amortized.
    let mut byte_queues: [VecDeque<QueueEntry>; Interpretation::COUNT] =
        core::array::from_fn(|_| VecDeque::new());
    let mut numeric_queues: [[VecDeque<QueueEntry>; 3]; Interpretation::COUNT] =
        core::array::from_fn(|_| core::array::from_fn(|_| VecDeque::new()));
    let mut alnum_queues: [[VecDeque<QueueEntry>; 2]; Interpretation::COUNT] =
        core::array::from_fn(|_| core::array::from_fn(|_| VecDeque::new()));
    // Kanji mode is only valid under the default interpretation or an explicit Shift JIS ECI.
    #[cfg(feature = "kanji")]
    let mut kanji_queues: [VecDeque<QueueEntry>; 2] = [VecDeque::new(), VecDeque::new()];

    for position in 0..=length {
        if position >= 1 {
            // Byte edges targeting Default and Latin1 measure the segment in characters.
            for target in [Interpretation::Default, Interpretation::Latin1] {
                let queue = &mut byte_queues[target.index()];

                while queue.front().is_some_and(|entry| entry.coordinate + byte_window < position) {
                    queue.pop_front();
                }

                if let Some(entry) = queue.front().copied() {
                    let switched = entry.active != target;

                    update_text(
                        &mut best[position][target.index()],
                        entry.step,
                        entry.start,
                        entry.active,
                        Mode::Byte,
                        target,
                        switched,
                        usize::from(switched) * eci_bits
                            + byte_overhead
                            + (position - entry.coordinate) * 8,
                    );
                }
            }

            // Byte edges targeting Utf8 measure the segment in UTF-8 bytes.
            {
                let coordinate = tables.offsets[position];
                let queue = &mut byte_queues[Interpretation::Utf8.index()];

                while queue.front().is_some_and(|entry| entry.coordinate + byte_window < coordinate)
                {
                    queue.pop_front();
                }

                if let Some(entry) = queue.front().copied() {
                    let switched = entry.active != Interpretation::Utf8;

                    update_text(
                        &mut best[position][Interpretation::Utf8.index()],
                        entry.step,
                        entry.start,
                        entry.active,
                        Mode::Byte,
                        Interpretation::Utf8,
                        switched,
                        usize::from(switched) * eci_bits
                            + byte_overhead
                            + (coordinate - entry.coordinate) * 8,
                    );
                }
            }

            // Byte edges targeting ShiftJis measure the segment in Shift JIS bytes.
            #[cfg(feature = "kanji")]
            {
                let coordinate = tables.sjis_prefix[position];
                let queue = &mut byte_queues[Interpretation::ShiftJis.index()];

                while queue.front().is_some_and(|entry| entry.coordinate + byte_window < coordinate)
                {
                    queue.pop_front();
                }

                if let Some(entry) = queue.front().copied() {
                    let switched = entry.active != Interpretation::ShiftJis;

                    update_text(
                        &mut best[position][Interpretation::ShiftJis.index()],
                        entry.step,
                        entry.start,
                        entry.active,
                        Mode::Byte,
                        Interpretation::ShiftJis,
                        switched,
                        usize::from(switched) * eci_bits
                            + byte_overhead
                            + (coordinate - entry.coordinate) * 8,
                    );
                }
            }

            // Numeric and alphanumeric segments never change the interpretation in force.
            for state in Interpretation::ALL {
                for queue in &mut numeric_queues[state.index()] {
                    while queue.front().is_some_and(|entry| entry.start + numeric_window < position)
                    {
                        queue.pop_front();
                    }

                    if let Some(entry) = queue.front().copied() {
                        update_text(
                            &mut best[position][state.index()],
                            entry.step,
                            entry.start,
                            state,
                            Mode::Numeric,
                            state,
                            false,
                            numeric_overhead + numeric_bits(position - entry.start),
                        );
                    }
                }

                let encoded = tables.alnum_prefix[position];

                for queue in &mut alnum_queues[state.index()] {
                    while queue
                        .front()
                        .is_some_and(|entry| entry.coordinate + alnum_window < encoded)
                    {
                        queue.pop_front();
                    }

                    if let Some(entry) = queue.front().copied() {
                        update_text(
                            &mut best[position][state.index()],
                            entry.step,
                            entry.start,
                            state,
                            Mode::Alphanumeric,
                            state,
                            false,
                            alnum_overhead + alphanumeric_bits(encoded - entry.coordinate),
                        );
                    }
                }
            }

            #[cfg(feature = "kanji")]
            {
                // Kanji edges targeting the default interpretation need no ECI header.
                let queue = &mut kanji_queues[0];

                while queue.front().is_some_and(|entry| entry.start + kanji_window < position) {
                    queue.pop_front();
                }

                if let Some(entry) = queue.front().copied() {
                    update_text(
                        &mut best[position][Interpretation::Default.index()],
                        entry.step,
                        entry.start,
                        Interpretation::Default,
                        Mode::Kanji,
                        Interpretation::Default,
                        false,
                        kanji_overhead + (position - entry.start) * 13,
                    );
                }

                // Kanji edges after any explicit ECI must declare Shift JIS first.
                let queue = &mut kanji_queues[1];

                while queue.front().is_some_and(|entry| entry.start + kanji_window < position) {
                    queue.pop_front();
                }

                if let Some(entry) = queue.front().copied() {
                    let switched = entry.active != Interpretation::ShiftJis;

                    update_text(
                        &mut best[position][Interpretation::ShiftJis.index()],
                        entry.step,
                        entry.start,
                        entry.active,
                        Mode::Kanji,
                        Interpretation::ShiftJis,
                        switched,
                        usize::from(switched) * eci_bits
                            + kanji_overhead
                            + (position - entry.start) * 13,
                    );
                }
            }
        }

        if position == length {
            break;
        }

        let default_bits = best[position][Interpretation::Default.index()].map(|step| step.bits);

        for state in Interpretation::ALL {
            let Some(step) = best[position][state.index()] else {
                continue;
            };

            // A default prefix replays any continuation of this state by paying its ECI header later, so the state is not a useful source.
            if state != Interpretation::Default
                && default_bits.is_some_and(|bits| step.bits >= bits + eci_bits)
            {
                continue;
            }

            let bits = step.bits as isize;

            // A start only enters a queue when its mode can extend through the character here.
            // A byte segment kept in the default interpretation can only continue a default prefix.
            if !tables.wide[position] {
                if state == Interpretation::Default {
                    push_entry(&mut byte_queues[Interpretation::Default.index()], QueueEntry {
                        cost: bits - 8 * position as isize,
                        switches: step.switches,
                        segments: step.segments,
                        start: position,
                        coordinate: position,
                        step,
                        active: state,
                    });
                } else {
                    // Declaring ECI 3 from the default interpretation is never cheaper than staying in it.
                    let switched = state != Interpretation::Latin1;

                    push_entry(&mut byte_queues[Interpretation::Latin1.index()], QueueEntry {
                        cost: bits + (usize::from(switched) * eci_bits) as isize
                            - 8 * position as isize,
                        switches: step.switches + usize::from(switched),
                        segments: step.segments + usize::from(switched),
                        start: position,
                        coordinate: position,
                        step,
                        active: state,
                    });
                }
            }

            {
                let switched = state != Interpretation::Utf8;
                let coordinate = tables.offsets[position];

                push_entry(&mut byte_queues[Interpretation::Utf8.index()], QueueEntry {
                    cost: bits + (usize::from(switched) * eci_bits) as isize
                        - 8 * coordinate as isize,
                    switches: step.switches + usize::from(switched),
                    segments: step.segments + usize::from(switched),
                    start: position,
                    coordinate,
                    step,
                    active: state,
                });
            }

            #[cfg(feature = "kanji")]
            if tables.sjis_ok[position] {
                let switched = state != Interpretation::ShiftJis;
                let coordinate = tables.sjis_prefix[position];

                push_entry(&mut byte_queues[Interpretation::ShiftJis.index()], QueueEntry {
                    cost: bits + (usize::from(switched) * eci_bits) as isize
                        - 8 * coordinate as isize,
                    switches: step.switches + usize::from(switched),
                    segments: step.segments + usize::from(switched),
                    start: position,
                    coordinate,
                    step,
                    active: state,
                });
            }

            if tables.digit[position] {
                push_entry(&mut numeric_queues[state.index()][position % 3], QueueEntry {
                    cost: 3 * bits - 10 * position as isize,
                    switches: step.switches,
                    segments: step.segments,
                    start: position,
                    coordinate: position,
                    step,
                    active: state,
                });
            }

            if tables.alnum_ok[position] {
                let coordinate = tables.alnum_prefix[position];

                push_entry(&mut alnum_queues[state.index()][coordinate & 1], QueueEntry {
                    cost: 2 * bits - 11 * coordinate as isize,
                    switches: step.switches,
                    segments: step.segments,
                    start: position,
                    coordinate,
                    step,
                    active: state,
                });
            }

            #[cfg(feature = "kanji")]
            if tables.kanji_ok[position] {
                if state == Interpretation::Default {
                    push_entry(&mut kanji_queues[0], QueueEntry {
                        cost: bits - 13 * position as isize,
                        switches: step.switches,
                        segments: step.segments,
                        start: position,
                        coordinate: position,
                        step,
                        active: state,
                    });
                } else {
                    // Declaring ECI 20 from the default interpretation is never cheaper than staying in it.
                    let switched = state != Interpretation::ShiftJis;

                    push_entry(&mut kanji_queues[1], QueueEntry {
                        cost: bits + (usize::from(switched) * eci_bits) as isize
                            - 13 * position as isize,
                        switches: step.switches + usize::from(switched),
                        segments: step.segments + usize::from(switched),
                        start: position,
                        coordinate: position,
                        step,
                        active: state,
                    });
                }
            }
        }

        // A character that a mode cannot represent ends every segment run of that mode.
        if !tables.digit[position] {
            for queues in &mut numeric_queues {
                for queue in queues {
                    queue.clear();
                }
            }
        }

        if !tables.alnum_ok[position] {
            for queues in &mut alnum_queues {
                for queue in queues {
                    queue.clear();
                }
            }
        }

        if tables.wide[position] {
            byte_queues[Interpretation::Default.index()].clear();
            byte_queues[Interpretation::Latin1.index()].clear();
        }

        #[cfg(feature = "kanji")]
        {
            if !tables.kanji_ok[position] {
                kanji_queues[0].clear();
                kanji_queues[1].clear();
            }

            if !tables.sjis_ok[position] {
                byte_queues[Interpretation::ShiftJis.index()].clear();
            }
        }
    }

    let mut final_choice: Option<(Interpretation, TextStep)> = None;

    // The iteration order makes ties prefer the default interpretation, then the lowest ECI usage.
    for interpretation in Interpretation::ALL {
        if let Some(step) = best[length][interpretation.index()]
            && final_choice.is_none_or(|(_, current)| {
                (step.bits, step.switches, step.segments)
                    < (current.bits, current.switches, current.segments)
            })
        {
            final_choice = Some((interpretation, step));
        }
    }

    let (final_interpretation, _) = final_choice
        .ok_or(EncodeError::DataTooLong {
            required_bits: usize::MAX, capacity_bits: 0
        })?;

    // Backtracking also recovers the interpretation selected before the first segment.
    let mut edges = Vec::new();
    let mut position = length;
    let mut interpretation = final_interpretation;

    while position != 0 {
        let step = best[position][interpretation.index()].expect("the final state is reachable");
        edges.push((step.previous_position, position, step));
        position = step.previous_position;
        interpretation = step.previous_interpretation;
    }

    edges.reverse();

    let mut result = Vec::new();

    if let Some(assignment) = interpretation.eci() {
        result.push(Segment::eci(assignment));
    }

    for (start, end, step) in edges {
        if step.switched {
            result.push(Segment::eci(
                step.interpretation.eci().expect("switched edges declare an explicit ECI"),
            ));
        }

        let slice = &text[tables.offsets[start]..tables.offsets[end]];

        result.push(match step.mode {
            Mode::Numeric => Segment::numeric(slice)?,
            Mode::Alphanumeric if fnc1 => Segment::fnc1_alphanumeric(slice.as_bytes())?,
            Mode::Alphanumeric => Segment::alphanumeric(slice)?,
            Mode::Byte => match step.interpretation {
                Interpretation::Default | Interpretation::Latin1 => {
                    let bytes: Vec<u8> = slice.chars().map(|character| character as u8).collect();
                    Segment::bytes(&bytes)
                },
                Interpretation::Utf8 => Segment::bytes(slice.as_bytes()),
                #[cfg(feature = "kanji")]
                Interpretation::ShiftJis => {
                    let (encoded, _, had_errors) = encoding_rs::SHIFT_JIS.encode(slice);

                    debug_assert!(!had_errors);

                    Segment::bytes(&encoded)
                },
            },
            #[cfg(feature = "kanji")]
            Mode::Kanji => Segment::kanji(slice)?,
            #[cfg(not(feature = "kanji"))]
            Mode::Kanji => unreachable!(),
            Mode::Eci => unreachable!(),
        });
    }
    Ok(result)
}

#[derive(Clone, Copy)]
struct ByteStep {
    bits:     usize,
    segments: usize,
    previous: usize,
    mode:     Mode,
}

// One queued byte-segmentation start with the end-dependent cost term removed.
#[derive(Clone, Copy)]
struct ByteQueueEntry {
    cost:       isize,
    segments:   usize,
    start:      usize,
    coordinate: usize,
    step:       ByteStep,
}

impl ByteQueueEntry {
    #[inline]
    const fn key(self) -> (isize, usize, usize) {
        (self.cost, self.segments, self.start)
    }
}

#[inline]
fn push_byte_entry(queue: &mut VecDeque<ByteQueueEntry>, entry: ByteQueueEntry) {
    while queue.back().is_some_and(|back| back.key() > entry.key()) {
        queue.pop_back();
    }

    queue.push_back(entry);
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

    // Prefix sums of encoded alphanumeric characters; FNC1 doubles a literal percent.
    let mut alnum_prefix = Vec::with_capacity(data.len() + 1);

    alnum_prefix.push(0usize);

    for &byte in data {
        let eligible = alphanumeric_value(byte).is_some() || (fnc1 && byte == 0x1D);
        let encoded = usize::from(eligible) * (1 + usize::from(fnc1 && byte == b'%'))
            + usize::from(!eligible);

        alnum_prefix
            .push(alnum_prefix.last().copied().expect("the prefix starts at zero") + encoded);
    }

    let numeric_overhead = profile.overhead_bits(Mode::Numeric);
    let alnum_overhead = profile.overhead_bits(Mode::Alphanumeric);
    let byte_overhead = profile.overhead_bits(Mode::Byte);

    let numeric_window = max_count(Mode::Numeric, profile);
    let alnum_window = max_count(Mode::Alphanumeric, profile);
    let byte_window = max_count(Mode::Byte, profile);

    // A monotonic queue per mode keeps the cheapest in-range segment start, making each edge O(1) amortized.
    let mut byte_queue: VecDeque<ByteQueueEntry> = VecDeque::new();
    let mut numeric_queues: [VecDeque<ByteQueueEntry>; 3] =
        core::array::from_fn(|_| VecDeque::new());
    let mut alnum_queues: [VecDeque<ByteQueueEntry>; 2] = core::array::from_fn(|_| VecDeque::new());

    for position in 0..=data.len() {
        if position >= 1 {
            while byte_queue.front().is_some_and(|entry| entry.start + byte_window < position) {
                byte_queue.pop_front();
            }

            if let Some(entry) = byte_queue.front().copied() {
                update_byte(
                    &mut best[position],
                    entry.step,
                    entry.start,
                    Mode::Byte,
                    byte_overhead + (position - entry.start) * 8,
                );
            }

            for queue in &mut numeric_queues {
                while queue.front().is_some_and(|entry| entry.start + numeric_window < position) {
                    queue.pop_front();
                }

                if let Some(entry) = queue.front().copied() {
                    update_byte(
                        &mut best[position],
                        entry.step,
                        entry.start,
                        Mode::Numeric,
                        numeric_overhead + numeric_bits(position - entry.start),
                    );
                }
            }

            let encoded = alnum_prefix[position];

            for queue in &mut alnum_queues {
                while queue.front().is_some_and(|entry| entry.coordinate + alnum_window < encoded) {
                    queue.pop_front();
                }

                if let Some(entry) = queue.front().copied() {
                    update_byte(
                        &mut best[position],
                        entry.step,
                        entry.start,
                        Mode::Alphanumeric,
                        alnum_overhead + alphanumeric_bits(encoded - entry.coordinate),
                    );
                }
            }
        }

        if position == data.len() {
            break;
        }

        let byte = data[position];
        let digit = byte.is_ascii_digit();
        let alnum = alphanumeric_value(byte).is_some() || (fnc1 && byte == 0x1D);

        // A start only enters a queue when its mode can extend through the byte at this position.
        if let Some(step) = best[position] {
            let bits = step.bits as isize;

            push_byte_entry(&mut byte_queue, ByteQueueEntry {
                cost: bits - 8 * position as isize,
                segments: step.segments,
                start: position,
                coordinate: position,
                step,
            });

            if digit {
                push_byte_entry(&mut numeric_queues[position % 3], ByteQueueEntry {
                    cost: 3 * bits - 10 * position as isize,
                    segments: step.segments,
                    start: position,
                    coordinate: position,
                    step,
                });
            }

            if alnum {
                let coordinate = alnum_prefix[position];

                push_byte_entry(&mut alnum_queues[coordinate & 1], ByteQueueEntry {
                    cost: 2 * bits - 11 * coordinate as isize,
                    segments: step.segments,
                    start: position,
                    coordinate,
                    step,
                });
            }
        }

        // A byte that a mode cannot represent ends every segment run of that mode.
        if !digit {
            for queue in &mut numeric_queues {
                queue.clear();
            }
        }

        if !alnum {
            for queue in &mut alnum_queues {
                queue.clear();
            }
        }
    }

    reconstruct_bytes(data, best, fnc1)
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

#[cfg(all(test, feature = "qr"))]
#[path = "optimizer_tests.rs"]
mod tests;
