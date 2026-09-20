use meowlive_domain::affinity::{apply_delta, calendar_day, gift_budget};
#[test]
fn fixed_day_and_gift_budget() {
    assert_eq!(calendar_day(0, 480), Some(0));
    assert_eq!(calendar_day(0, -480), Some(-1));
    assert_eq!(calendar_day(0, 841), None);
    assert_eq!(gift_budget(100), 69);
    assert_eq!(gift_budget(u64::MAX), 500);
    assert_eq!(apply_delta(99950, 200), (100000, 50));
    assert_eq!(apply_delta(10, -200), (0, -10));
}
