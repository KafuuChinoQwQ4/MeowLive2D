//! 可审计的毫分积分与固定偏移日历规则。
pub fn calendar_day(timestamp_ms: u64, offset_minutes: i32) -> Option<i64> {
    if !(-840..=840).contains(&offset_minutes) {
        return None;
    }
    let ms = i64::try_from(timestamp_ms).ok()?;
    Some(
        ms.checked_add(i64::from(offset_minutes) * 60_000)?
            .div_euclid(86_400_000),
    )
}
/// 确认付费价值以人民币分计；仅计算累计预算，不推测平台价格单位。
pub fn gift_budget(value_cents: u64) -> i32 {
    (100.0 * (1.0 + value_cents as f64 / 100.0).ln())
        .floor()
        .min(500.0) as i32
}
pub fn apply_delta(current: i32, computed: i32) -> (i32, i32) {
    let next = (i64::from(current) + i64::from(computed)).clamp(0, 100_000) as i32;
    (next, next - current)
}
