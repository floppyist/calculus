use crate::ui::format::{format_scale, now_hms};

#[test]
fn test_format_scale_rounds() {
    assert_eq!(format_scale(0.0), "0");
    assert_eq!(format_scale(2500.0), "2500");
    assert_eq!(format_scale(-1.099943939008386), "-1.1");
}

#[test]
fn test_now_hms_format() {
    let stamp = now_hms();

    assert_eq!(stamp.len(), 8);
    assert_eq!(stamp.chars().filter(|c| *c == ':').count(), 2);
}
