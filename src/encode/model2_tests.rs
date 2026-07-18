// Executable checks for the source ambiguities recorded in iso-iec-18004-2024/09_known_issues_and_ambiguities.md.

use super::*;

// the Annex I example resolves to data mask 010, so ECL M with mask 2 yields format value 0x5E7C.
#[test]
fn ki_001_annex_i_format_value_matches() {
    assert_eq!(0x5E7C, format_bits(QrErrorCorrection::Medium, 2));
}

// the Annex C example left operand reads 00101, so ECL M with mask 5 yields format value 0x40CE.
#[test]
fn ki_002_annex_c_format_value_matches() {
    assert_eq!(0x40CE, format_bits(QrErrorCorrection::Medium, 5));
}

// the 45% to 55% dark ratio band scores zero with inclusive endpoints.
#[test]
fn ki_005_n4_zero_band_endpoints_are_inclusive() {
    assert_eq!(0, n4_penalty(180, 400));
    assert_eq!(0, n4_penalty(200, 400));
    assert_eq!(0, n4_penalty(220, 400));
}

// one module past either endpoint leaves the zero band and scores one step.
#[test]
fn ki_005_n4_scores_one_step_just_outside_the_zero_band() {
    assert_eq!(PENALTY_N4, n4_penalty(179, 400));
    assert_eq!(PENALTY_N4, n4_penalty(221, 400));
}

// each further 5% band adds one step, and its endpoints stay with the lower step.
#[test]
fn ki_005_n4_steps_grow_by_five_percent_bands() {
    assert_eq!(PENALTY_N4, n4_penalty(160, 400));
    assert_eq!(2 * PENALTY_N4, n4_penalty(159, 400));
    assert_eq!(9 * PENALTY_N4, n4_penalty(0, 400));
    assert_eq!(9 * PENALTY_N4, n4_penalty(400, 400));
}
