use alloc::{string::String, vec, vec::Vec};

use super::*;

const INFINITY: usize = usize::MAX / 2;

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
                Interpretation::Default | Interpretation::Latin1 => {
                    for &byte in segment.source_bytes() {
                        result.push(char::from(byte));
                    }
                },
                Interpretation::Utf8 => result.push_str(
                    core::str::from_utf8(segment.source_bytes())
                        .expect("UTF-8 byte segments hold valid UTF-8"),
                ),
                #[cfg(feature = "kanji")]
                Interpretation::ShiftJis => {
                    let (decoded, _, had_errors) =
                        encoding_rs::SHIFT_JIS.decode(segment.source_bytes());

                    assert!(!had_errors);
                    result.push_str(&decoded);
                },
            },
            Mode::Kanji => {
                #[cfg(feature = "kanji")]
                {
                    // Kanji mode is only legal under the default or Shift JIS interpretation.
                    assert!(matches!(
                        interpretation,
                        Interpretation::Default | Interpretation::ShiftJis
                    ));

                    let (decoded, _, had_errors) =
                        encoding_rs::SHIFT_JIS.decode(segment.source_bytes());

                    assert!(!had_errors);
                    result.push_str(&decoded);
                }
                #[cfg(not(feature = "kanji"))]
                panic!("Kanji segments need the kanji feature");
            },
        }
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
fn reference_text_bits(text: &str, profile: Profile, fnc1: bool, force_initial_eci: bool) -> usize {
    let tables = TextTables::new(text, fnc1);
    let length = tables.offsets.len() - 1;
    let eci_bits = profile.eci_bits();
    let mut best = vec![[INFINITY; Interpretation::COUNT]; length + 1];

    if force_initial_eci {
        for interpretation in Interpretation::ALL {
            if interpretation != Interpretation::Default {
                best[0][interpretation.index()] = eci_bits;
            }
        }
    } else {
        best[0][Interpretation::Default.index()] = 0;
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
                let target = if state == Interpretation::Default {
                    Interpretation::Default
                } else {
                    Interpretation::Latin1
                };
                let switch = usize::from(state != target) * eci_bits;

                end = start;

                while end < length
                    && !tables.wide[end]
                    && end - start < max_count(Mode::Byte, profile)
                {
                    end += 1;

                    relax(
                        &mut best[end][target.index()],
                        base + switch + profile.overhead_bits(Mode::Byte) + (end - start) * 8,
                    );
                }
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

                let target = if state == Interpretation::Default {
                    Interpretation::Default
                } else {
                    Interpretation::ShiftJis
                };
                let switch = usize::from(state != target) * eci_bits;

                end = start;

                while end < length
                    && tables.kanji_ok[end]
                    && end - start < max_count(Mode::Kanji, profile)
                {
                    end += 1;

                    relax(
                        &mut best[end][target.index()],
                        base + switch + profile.overhead_bits(Mode::Kanji) + (end - start) * 13,
                    );
                }
            }
        }
    }

    best[length].iter().copied().min().expect("at least one state exists")
}

fn verify_text(text: &str, profile: Profile, fnc1: bool, force_initial_eci: bool) {
    let plan = super::text(text, profile, fnc1, force_initial_eci).expect("text always has a plan");
    let expected = reference_text_bits(text, profile, fnc1, force_initial_eci);

    assert_eq!(
        expected,
        plan_bits(profile, &plan),
        "bits differ for {text:?} fnc1={fnc1} force={force_initial_eci}"
    );
    assert_eq!(text, decode_plan(&plan, fnc1), "readback differs for {text:?}");

    if force_initial_eci {
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
    let mut result = vec!['7', 'K', '%', ' ', 'é', '😀', '\\', '\u{1D}'];

    #[cfg(feature = "kanji")]
    result.extend(['点', 'ﾃ', '¥', '−', '－']);

    result
}

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
            verify_text(text, profile, true, false);
            verify_bytes(text.as_bytes(), profile, true);
        }
    }
}

// Every character the Shift JIS conversion accepts must decode back to itself.
#[cfg(feature = "kanji")]
#[test]
fn shift_jis_encoding_round_trips_every_accepted_character() {
    for value in 0..=u32::from(char::MAX) {
        let Some(character) = char::from_u32(value) else {
            continue;
        };
        let Some((bytes, length)) = super::super::shift_jis_encoding(character) else {
            continue;
        };

        let (decoded, _, had_errors) = encoding_rs::SHIFT_JIS.decode(&bytes[..length]);
        let mut buffer = [0; 4];
        let expected: &str = character.encode_utf8(&mut buffer);

        assert!(!had_errors, "{character:?} decodes with errors");
        assert_eq!(expected, decoded.as_ref(), "{character:?} does not round-trip");
    }
}

#[cfg(feature = "kanji")]
#[test]
fn minus_sign_keeps_its_character_across_eci_transitions() {
    for profile in profiles() {
        for text in ["−", "－", "日本語−日本語", "ﾃｽﾄ−ﾃｽﾄ"] {
            verify_text(text, profile, false, false);
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
                for force_initial_eci in [false, true] {
                    verify_text(input, profile, fnc1, force_initial_eci);
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

        verify_text(&input, profiles()[0], round % 2 == 0, round % 4 >= 2);
    }
}

// A run longer than the character count limit is split into multiple optimal segments.
#[test]
fn text_plans_split_segments_at_character_count_limits() {
    let input = "9".repeat(1100);

    verify_text(&input, profiles()[0], false, false);
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
