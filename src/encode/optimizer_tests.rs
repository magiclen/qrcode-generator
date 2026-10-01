use alloc::{string::String, vec, vec::Vec};

use super::*;

const INFINITY: usize = usize::MAX / 2;
// The longest input whose every prefix is checked against the reference.
const PREFIX_CHECK_LIMIT: usize = 12;

#[inline]
fn relax(slot: &mut usize, cost: usize) {
    if cost < *slot {
        *slot = cost;
    }
}

// Computes the exact bit length of an emitted plan using the per-segment cost model.
fn plan_bits(profile: Profile, segments: &[Segment]) -> usize {
    segments
        .iter()
        .map(|segment| {
            usize::from(profile.mode_bits)
                + usize::from(profile.cci_bits(segment.mode))
                + segment.bits.len()
        })
        .sum()
}

// Replays the plan the way a strict AIM ECI reader would and returns the decoded text.
fn decode_plan(segments: &[Segment], fnc1: bool) -> String {
    let mut interpretation = Interpretation::Default;
    let mut result = String::new();
    // Kanji mode without an ECI header only reads as Shift JIS when the whole symbol has no ECI header.
    let eci_free = segments.iter().all(|segment| segment.mode != Mode::Eci);
    let legacy = eci_free && segments.iter().any(|segment| segment.mode == Mode::Kanji);

    for segment in segments {
        match segment.mode {
            Mode::Eci => {
                interpretation = if *segment == Segment::eci(EciAssignment::ISO_8859_1) {
                    Interpretation::Latin1
                } else if *segment == Segment::eci(EciAssignment::UTF_8) {
                    Interpretation::Utf8
                } else {
                    #[cfg(feature = "kanji")]
                    {
                        assert_eq!(Segment::eci(EciAssignment::SHIFT_JIS), *segment);
                        Interpretation::ShiftJis
                    }
                    #[cfg(not(feature = "kanji"))]
                    panic!("unexpected ECI segment");
                };
            },
            // Numeric and alphanumeric characters read identically in every declared charset.
            Mode::Numeric => {
                for &byte in segment.source_bytes() {
                    result.push(char::from(byte));
                }
            },
            Mode::Alphanumeric => {
                for byte in decode_alphanumeric(segment, fnc1) {
                    result.push(char::from(byte));
                }
            },
            Mode::Byte => match interpretation {
                Interpretation::Utf8 => result.push_str(
                    core::str::from_utf8(segment.source_bytes())
                        .expect("UTF-8 byte segments hold valid UTF-8"),
                ),
                #[cfg(feature = "kanji")]
                Interpretation::ShiftJis => {
                    result.push_str(&decode_shift_jis(segment.source_bytes()))
                },
                // The default and Latin-1 interpretations hold one Table 6 byte per character.
                _ => {
                    for &byte in segment.source_bytes() {
                        // Table 6 leaves 80 to 9F undefined, and E0 to EB look like Shift JIS lead bytes next to Kanji mode data.
                        assert!(
                            !(0x80..=0x9F).contains(&byte),
                            "undefined Table 6 byte {byte:#04X}"
                        );
                        assert!(
                            !legacy || !(0xE0..=0xEB).contains(&byte),
                            "lead byte {byte:#04X} next to Kanji"
                        );
                        result.push(char::from(byte));
                    }
                },
            },
            Mode::Kanji => {
                #[cfg(feature = "kanji")]
                {
                    // Kanji mode is only legal in a symbol without ECI headers or under the Shift JIS interpretation.
                    assert!(
                        legacy || interpretation == Interpretation::ShiftJis,
                        "Kanji mode before or under a non Shift JIS ECI"
                    );

                    result.push_str(&decode_shift_jis(segment.source_bytes()));
                }
                #[cfg(not(feature = "kanji"))]
                panic!("Kanji segments need the kanji feature");
            },
        }
    }

    result
}

// Reads Shift JIS bytes with the JIS8 single bytes of ISO/IEC 18004 and checks every pair against the agreed JIS X 0208 set.
#[cfg(feature = "kanji")]
fn decode_shift_jis(bytes: &[u8]) -> String {
    let mut result = String::new();
    let mut index = 0;

    while index < bytes.len() {
        let length = if matches!(bytes[index], 0x81..=0x9F | 0xE0..=0xFC) { 2 } else { 1 };
        let chunk = &bytes[index..index + length];

        match *chunk {
            // WHATWG reads these as backslash and tilde, but JIS8 assigns the yen sign and the overline.
            [0x5C] => result.push('¥'),
            [0x7E] => result.push('\u{203E}'),
            [byte] => {
                assert!(
                    matches!(byte, 0x00..=0x7F | 0xA1..=0xDF),
                    "undefined JIS8 byte {byte:#04X}"
                );
                result.push_str(&encoding_rs::SHIFT_JIS.decode_without_bom_handling(chunk).0);
            },
            [lead, trail] => {
                let value = u16::from_be_bytes([lead, trail]);

                assert!(
                    matches!(value, 0x8140..=0x84BE | 0x889F..=0x9FFC | 0xE040..=0xEAA4)
                        && !matches!(
                            value,
                            0x815C | 0x815F | 0x8160 | 0x8161 | 0x817C | 0x8191 | 0x8192 | 0x81CA
                        ),
                    "disputed or non-JIS X 0208 pair {value:#06X}"
                );

                let (decoded, had_errors) =
                    encoding_rs::SHIFT_JIS.decode_without_bom_handling(chunk);

                assert!(!had_errors);
                result.push_str(&decoded);
            },
            _ => unreachable!(),
        }

        index += length;
    }

    result
}

fn decode_alphanumeric(segment: &Segment, fnc1: bool) -> Vec<u8> {
    const ALPHABET: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:";
    let mut offset = 0;
    let mut read = |count| {
        let mut value = 0usize;
        for _ in 0..count {
            value = value * 2 + usize::from(segment.bits.bit(offset));
            offset += 1;
        }
        value
    };
    let mut encoded = Vec::new();
    for _ in 0..segment.character_count / 2 {
        let value = read(11);
        encoded.extend_from_slice(&[ALPHABET[value / 45], ALPHABET[value % 45]]);
    }
    if !segment.character_count.is_multiple_of(2) {
        encoded.push(ALPHABET[read(6)]);
    }
    assert_eq!(segment.bits.len(), offset);

    let mut result = Vec::new();
    let mut position = 0;
    while position < encoded.len() {
        let byte = encoded[position];
        if fnc1 && byte == b'%' {
            if encoded.get(position + 1) == Some(&b'%') {
                result.push(b'%');
                position += 1;
            } else {
                result.push(0x1D);
            }
        } else {
            result.push(byte);
        }
        position += 1;
    }
    result
}

// A direct quadratic reference that explores every legal edge, confirming bit optimality.
fn reference_text_bits(text: &str, profile: Profile, fnc1: bool, initial: Start) -> usize {
    let tables = TextTables::new(text, fnc1);
    let length = tables.offsets.len() - 1;
    let eci_bits = profile.eci_bits();
    let mut best = vec![[INFINITY; Interpretation::COUNT]; length + 1];

    match initial {
        Start::Free => {
            best[0][Interpretation::Default.index()] = 0;

            #[cfg(feature = "kanji")]
            {
                best[0][Interpretation::Legacy.index()] = 0;
            }
        },
        Start::Default | Start::DefaultEciFree => best[0][Interpretation::Default.index()] = 0,
        #[cfg(feature = "kanji")]
        Start::Legacy => best[0][Interpretation::Legacy.index()] = 0,
        Start::Explicit => {
            for interpretation in Interpretation::ALL {
                if interpretation.eci().is_some() {
                    best[0][interpretation.index()] = eci_bits;
                }
            }
        },
    }

    for start in 0..length {
        for state in Interpretation::ALL {
            let base = best[start][state.index()];

            if base >= INFINITY {
                continue;
            }

            let mut end = start;

            while end < length
                && tables.digit[end]
                && end - start < max_count(Mode::Numeric, profile)
            {
                end += 1;

                relax(
                    &mut best[end][state.index()],
                    base + profile.overhead_bits(Mode::Numeric) + numeric_bits(end - start),
                );
            }

            end = start;

            while end < length
                && tables.alnum_ok[end]
                && (!fnc1
                    || end == start
                    || text.as_bytes()[tables.offsets[end - 1]] != 0x1D
                    || !matches!(text.as_bytes()[tables.offsets[end]], 0x1D | b'%'))
                && tables.alnum_prefix[end + 1] - tables.alnum_prefix[start]
                    <= max_count(Mode::Alphanumeric, profile)
            {
                end += 1;

                relax(
                    &mut best[end][state.index()],
                    base + profile.overhead_bits(Mode::Alphanumeric)
                        + alphanumeric_bits(tables.alnum_prefix[end] - tables.alnum_prefix[start]),
                );
            }

            {
                let target = if state.eci().is_none() { state } else { Interpretation::Latin1 };
                let switch = usize::from(state != target) * eci_bits;
                #[cfg(feature = "kanji")]
                let byte_ok = if state.is_legacy() { &tables.legacy_ok } else { &tables.latin1_ok };
                #[cfg(not(feature = "kanji"))]
                let byte_ok = &tables.latin1_ok;

                end = start;

                while end < length && byte_ok[end] && end - start < max_count(Mode::Byte, profile) {
                    end += 1;

                    relax(
                        &mut best[end][target.index()],
                        base + switch + profile.overhead_bits(Mode::Byte) + (end - start) * 8,
                    );
                }
            }

            // A symbol without ECI headers never switches to an explicit interpretation.
            if state.is_legacy() {
                #[cfg(feature = "kanji")]
                {
                    end = start;

                    while end < length
                        && tables.kanji_ok[end]
                        && end - start < max_count(Mode::Kanji, profile)
                    {
                        end += 1;

                        relax(
                            &mut best[end][state.index()],
                            base + profile.overhead_bits(Mode::Kanji) + (end - start) * 13,
                        );
                    }
                }

                continue;
            }

            // Every remaining edge leaves the default interpretation through an ECI header.
            if initial == Start::DefaultEciFree {
                continue;
            }

            {
                let switch = usize::from(state != Interpretation::Utf8) * eci_bits;

                end = start;

                while end < length
                    && tables.offsets[end + 1] - tables.offsets[start]
                        <= max_count(Mode::Byte, profile)
                {
                    end += 1;

                    relax(
                        &mut best[end][Interpretation::Utf8.index()],
                        base + switch
                            + profile.overhead_bits(Mode::Byte)
                            + (tables.offsets[end] - tables.offsets[start]) * 8,
                    );
                }
            }

            #[cfg(feature = "kanji")]
            {
                let switch = usize::from(state != Interpretation::ShiftJis) * eci_bits;

                end = start;

                while end < length
                    && tables.sjis_ok[end]
                    && tables.sjis_prefix[end + 1] - tables.sjis_prefix[start]
                        <= max_count(Mode::Byte, profile)
                {
                    end += 1;

                    relax(
                        &mut best[end][Interpretation::ShiftJis.index()],
                        base + switch
                            + profile.overhead_bits(Mode::Byte)
                            + (tables.sjis_prefix[end] - tables.sjis_prefix[start]) * 8,
                    );
                }

                let switch = usize::from(state != Interpretation::ShiftJis) * eci_bits;

                end = start;

                while end < length
                    && tables.kanji_ok[end]
                    && end - start < max_count(Mode::Kanji, profile)
                {
                    end += 1;

                    relax(
                        &mut best[end][Interpretation::ShiftJis.index()],
                        base + switch + profile.overhead_bits(Mode::Kanji) + (end - start) * 13,
                    );
                }
            }
        }
    }

    best[length].iter().copied().min().expect("at least one state exists")
}

fn verify_text(text: &str, profile: Profile, fnc1: bool, start: Start) {
    let expected = reference_text_bits(text, profile, fnc1, start);

    let costs = super::text_costs(text, profile, fnc1, start);

    assert_eq!((expected < INFINITY).then_some(expected), costs.last().copied().flatten());

    // Every prefix cost of a short input matches the optimum of that prefix on its own; longer inputs would make the quadratic reference too slow.
    if costs.len() - 1 <= PREFIX_CHECK_LIMIT {
        for (count, &cost) in costs.iter().enumerate() {
            let end = text.char_indices().nth(count).map_or(text.len(), |(offset, _)| offset);
            let prefix = reference_text_bits(&text[..end], profile, fnc1, start);

            assert_eq!((prefix < INFINITY).then_some(prefix), cost, "prefix {count} of {text:?}");
        }
    }

    // Starts without ECI headers cannot represent every character.
    let Ok(plan) = super::text(text, profile, fnc1, start) else {
        assert!(expected >= INFINITY, "no plan for {text:?} fnc1={fnc1} start={start:?}");
        return;
    };

    assert_eq!(
        expected,
        plan_bits(profile, &plan),
        "bits differ for {text:?} fnc1={fnc1} start={start:?}"
    );
    assert_eq!(text, decode_plan(&plan, fnc1), "readback differs for {text:?}");

    if start == Start::DefaultEciFree {
        assert!(plan.iter().all(|segment| segment.mode != Mode::Eci));
    }

    if start == Start::Explicit {
        assert!(matches!(plan.first(), Some(segment) if segment.mode == Mode::Eci));
    }
}

fn reference_bytes_bits(data: &[u8], profile: Profile, fnc1: bool) -> usize {
    let mut alnum_prefix = vec![0usize];

    for &byte in data {
        let eligible = alphanumeric_value(byte).is_some() || (fnc1 && byte == 0x1D);
        let encoded = usize::from(eligible) * (1 + usize::from(fnc1 && byte == b'%'))
            + usize::from(!eligible);

        alnum_prefix
            .push(alnum_prefix.last().copied().expect("the prefix starts at zero") + encoded);
    }

    let mut best = vec![INFINITY; data.len() + 1];

    best[0] = 0;

    for start in 0..data.len() {
        let base = best[start];

        if base >= INFINITY {
            continue;
        }

        let mut end = start;

        while end < data.len() && end - start < max_count(Mode::Byte, profile) {
            end += 1;

            relax(&mut best[end], base + profile.overhead_bits(Mode::Byte) + (end - start) * 8);
        }

        end = start;

        while end < data.len()
            && data[end].is_ascii_digit()
            && end - start < max_count(Mode::Numeric, profile)
        {
            end += 1;

            relax(
                &mut best[end],
                base + profile.overhead_bits(Mode::Numeric) + numeric_bits(end - start),
            );
        }

        end = start;

        while end < data.len()
            && (alphanumeric_value(data[end]).is_some() || (fnc1 && data[end] == 0x1D))
            && (!fnc1 || end == start || data[end - 1] != 0x1D || !matches!(data[end], 0x1D | b'%'))
            && alnum_prefix[end + 1] - alnum_prefix[start] <= max_count(Mode::Alphanumeric, profile)
        {
            end += 1;

            relax(
                &mut best[end],
                base + profile.overhead_bits(Mode::Alphanumeric)
                    + alphanumeric_bits(alnum_prefix[end] - alnum_prefix[start]),
            );
        }
    }

    best[data.len()]
}

fn verify_bytes(data: &[u8], profile: Profile, fnc1: bool) {
    let costs = super::byte_costs(data, profile, fnc1);

    assert_eq!(Some(reference_bytes_bits(data, profile, fnc1)), costs.last().copied().flatten());

    // Every prefix cost of a short input matches the optimum of that prefix on its own; longer inputs would make the quadratic reference too slow.
    if data.len() <= PREFIX_CHECK_LIMIT {
        for (length, &cost) in costs.iter().enumerate() {
            assert_eq!(
                Some(reference_bytes_bits(&data[..length], profile, fnc1)),
                cost,
                "prefix {length} of {data:?}"
            );
        }
    }

    let plan = super::bytes(data, profile, fnc1).expect("bytes always have a plan");

    assert_eq!(
        reference_bytes_bits(data, profile, fnc1),
        plan_bits(profile, &plan),
        "bits differ for {data:?} fnc1={fnc1}"
    );

    let mut readback = Vec::new();

    for segment in &plan {
        if segment.mode == Mode::Alphanumeric {
            readback.extend(decode_alphanumeric(segment, fnc1));
        } else {
            readback.extend_from_slice(segment.source_bytes());
        }
    }

    assert_eq!(data, readback, "readback differs for {data:?}");
}

fn text_alphabet() -> Vec<char> {
    #[allow(unused_mut)]
    let mut result = vec!['7', 'K', '%', ' ', 'é', '😀', '\\', '\u{1D}', '\u{85}'];

    #[cfg(feature = "kanji")]
    result.extend(['点', 'ﾃ', '¥', '−', '－', '×']);

    result
}

fn starts() -> Vec<Start> {
    #[allow(unused_mut)]
    let mut result = vec![Start::Free, Start::Default, Start::DefaultEciFree, Start::Explicit];

    #[cfg(feature = "kanji")]
    result.push(Start::Legacy);

    result
}

#[cfg_attr(not(feature = "qr"), allow(clippy::vec_init_then_push))]
fn profiles() -> Vec<Profile> {
    let mut profiles = Vec::new();

    #[cfg(feature = "qr")]
    profiles.extend([
        Profile::qr(QrVersion::new(1).expect("version 1 is valid")),
        Profile::qr(QrVersion::new(27).expect("version 27 is valid")),
    ]);
    #[cfg(feature = "rmqr")]
    profiles.push(Profile::rmqr(super::super::rmqr::cci(super::super::RmqrVersion::R7x43)));

    profiles
}

#[test]
fn fnc1_adjacent_separators_and_percents_round_trip() {
    #[allow(unused_mut)]
    let mut profiles = profiles();

    // Every distinct rMQR character count indicator profile joins in, because the shortest ones force extra segments.
    #[cfg(feature = "rmqr")]
    for version in super::super::RmqrVersion::ALL {
        let profile = Profile::rmqr(super::super::rmqr::cci(version));

        if !profiles.contains(&profile) {
            profiles.push(profile);
        }
    }

    for profile in profiles {
        for text in ["ABC\u{1D}%DEF", "A\u{1D}\u{1D}B", "%\u{1D}", "%%"] {
            verify_text(text, profile, true, Start::Free);
            verify_bytes(text.as_bytes(), profile, true);
        }
    }
}

// Every character the Shift JIS conversion accepts must decode back to itself under the JIS8 and JIS X 0208 rules.
#[cfg(feature = "kanji")]
#[test]
fn shift_jis_encoding_round_trips_every_accepted_character() {
    let mut counts = [0usize; 3];

    for value in 0..=u32::from(char::MAX) {
        let Some(character) = char::from_u32(value) else {
            continue;
        };
        let Some((bytes, length)) = super::super::shift_jis_encoding(character) else {
            continue;
        };

        let mut buffer = [0; 4];
        let expected: &str = character.encode_utf8(&mut buffer);

        assert_eq!(
            expected,
            decode_shift_jis(&bytes[..length]),
            "{character:?} does not round-trip"
        );

        counts[if length == 2 { 2 } else { usize::from(!character.is_ascii()) }] += 1;
    }

    // ASCII without backslash and tilde, the yen sign, the overline and 63 half-width katakana, then the agreed JIS X 0208 pairs.
    assert_eq!([126, 65, 6871], counts);
}

// Disputed JIS X 0208 positions and characters outside Table 6 fall back to other interpretations and keep their meaning.
#[cfg(feature = "kanji")]
#[test]
fn disputed_and_undefined_characters_keep_their_meaning() {
    for profile in profiles() {
        for text in ["−", "－", "～", "①", "髙", "\u{85}", "日本語−日本語", "ﾃｽﾄ－ﾃｽﾄ", "ﾃｽﾄ¥‾ﾃｽﾄ"]
        {
            verify_text(text, profile, false, Start::Free);
        }
    }
}

// Every short input over a charset covering all modes and interpretations is bit-optimal.
#[test]
fn text_plans_are_bit_optimal_for_all_short_inputs() {
    let alphabet = text_alphabet();
    let mut inputs = vec![String::new()];
    let mut layer = vec![String::new()];

    for _ in 0..3 {
        let mut next = Vec::new();

        for prefix in &layer {
            for &character in &alphabet {
                let mut input = prefix.clone();

                input.push(character);
                next.push(input);
            }
        }

        inputs.extend(next.iter().cloned());
        layer = next;
    }

    let profiles = profiles();

    for input in &inputs {
        for &profile in &profiles {
            for fnc1 in [false, true] {
                for start in starts() {
                    verify_text(input, profile, fnc1, start);
                }
            }
        }
    }
}

// Longer pseudo-random inputs stay bit-optimal and read back exactly.
#[test]
fn text_plans_are_bit_optimal_for_random_inputs() {
    let alphabet = text_alphabet();
    let mut state = 0x243F_6A88_85A3_08D3u64;

    for round in 0..60usize {
        let mut input = String::new();

        for _ in 0..40 {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            input.push(alphabet[(state >> 33) as usize % alphabet.len()]);
        }

        let starts = starts();
        let start = starts[round / 2 % starts.len()];

        verify_text(&input, profiles()[0], round % 2 == 0, start);
    }
}

// A run longer than the character count limit is split into multiple optimal segments.
#[test]
fn text_plans_split_segments_at_character_count_limits() {
    let input = "9".repeat(1100);

    verify_text(&input, profiles()[0], false, Start::Free);
}

// Every short byte input over a charset covering all modes is bit-optimal.
#[test]
fn byte_plans_are_bit_optimal_for_all_short_inputs() {
    let alphabet = [b'7', b'A', b'%', 0x1D, 0xFF];
    let mut inputs = vec![Vec::new()];
    let mut layer = vec![Vec::new()];

    for _ in 0..3 {
        let mut next = Vec::new();

        for prefix in &layer {
            for &byte in &alphabet {
                let mut input = prefix.clone();

                input.push(byte);
                next.push(input);
            }
        }

        inputs.extend(next.iter().cloned());
        layer = next;
    }

    let profiles = profiles();

    for input in &inputs {
        for &profile in &profiles {
            for fnc1 in [false, true] {
                verify_bytes(input, profile, fnc1);
            }
        }
    }
}

// Longer pseudo-random byte inputs stay bit-optimal and read back exactly.
#[test]
fn byte_plans_are_bit_optimal_for_random_inputs() {
    let alphabet = [b'7', b'0', b'A', b'%', b' ', 0x1D, 0x80, 0xFF];
    let mut state = 0x4528_21E6_38D0_1377u64;

    for round in 0..60usize {
        let mut input = Vec::new();

        for _ in 0..60 {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            input.push(alphabet[(state >> 33) as usize % alphabet.len()]);
        }

        verify_bytes(&input, profiles()[0], round % 2 == 0);
    }
}
