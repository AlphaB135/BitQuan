use super::*;

#[test]
fn subsidy_initial_and_halving() {
    let rs = RewardSchedule::phase3_defaults();
    assert_eq!(rs.subsidy_at_height(0), 5_000_000_000);
    assert_eq!(rs.subsidy_at_height(210_000 - 1), 5_000_000_000);
    assert_eq!(rs.subsidy_at_height(210_000), 2_500_000_000);
    assert_eq!(rs.subsidy_at_height(210_000 * 6), 78_125_000);
}

#[test]
fn subsidy_tail_emission_after_seven_halvings() {
    let rs = RewardSchedule::phase3_defaults();
    assert_eq!(rs.subsidy_at_height(210_000 * 7), 50_000_000);
    assert_eq!(rs.subsidy_at_height(210_000 * 1000), 50_000_000);
}
