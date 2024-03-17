use chrono::{FixedOffset, TimeZone};

pub fn from_mail_parser_to_chrono_datetime(
    dt: &mail_parser::DateTime,
) -> Option<chrono::DateTime<FixedOffset>> {
    let tz_secs = (dt.tz_hour as i32) * 3600 + (dt.tz_minute as i32) * 60;
    let tz_sign = if dt.tz_before_gmt { -1 } else { 1 };

    FixedOffset::east_opt(tz_sign * tz_secs)?
        .with_ymd_and_hms(
            dt.year as i32,
            dt.month as u32,
            dt.day as u32,
            dt.hour as u32,
            dt.minute as u32,
            dt.second as u32,
        )
        .earliest()
}
