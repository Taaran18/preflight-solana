use std::time::{SystemTime, UNIX_EPOCH};

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub fn now_secs() -> i64 {
    (now_ms() / 1000) as i64
}

pub fn parse_iso_secs(value: &str) -> Option<i64> {
    let num = |start: usize, end: usize| value.get(start..end)?.parse::<i64>().ok();
    let year = num(0, 4)?;
    let month = num(5, 7)?;
    let day = num(8, 10)?;
    let hour = num(11, 13).unwrap_or(0);
    let minute = num(14, 16).unwrap_or(0);
    let second = num(17, 19).unwrap_or(0);
    Some(days_from_civil(year, month, day) * 86_400 + hour * 3_600 + minute * 60 + second)
}

pub fn age_days(value: Option<&str>) -> Option<f64> {
    let created = parse_iso_secs(value?)?;
    let delta = now_secs() - created;
    if delta < 0 {
        return Some(0.0);
    }
    Some(delta as f64 / 86_400.0)
}

fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}
