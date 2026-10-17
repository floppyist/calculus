use crate::core::environment::Outcome;

pub fn format_number(value: f64) -> String {
    if value.is_nan() {
        "NaN".to_string()
    } else if value.is_infinite() {
        if value.is_sign_negative() {
            "-inf".to_string()
        } else {
            "inf".to_string()
        }
    } else if value == value.trunc() && value.abs() < 1e15 {
        format!("{}", value as i64)
    } else {
        format!("{}", value)
    }
}

// Compact axis label rounded to a few significant digits
pub fn format_scale(value: f64) -> String {
    if !value.is_finite() {
        return format_number(value);
    }

    if value == 0.0 {
        return "0".to_string();
    }

    let magnitude = value.abs().log10().floor();
    let decimals = (3.0 - magnitude).clamp(0.0, 6.0) as usize;
    let mut text = format!("{:.*}", decimals, value);

    if text.contains('.') {
        while text.ends_with('0') {
            text.pop();
        }

        if text.ends_with('.') {
            text.pop();
        }
    }

    text
}

pub fn format_outcome(outcome: Outcome) -> String {
    match outcome {
        Outcome::Value(value) => format!("= {}", format_number(value)),
        Outcome::Exact(value) => format!("= {}", value),
        Outcome::VariableDefined(definition) => definition,
        Outcome::FunctionDefined(definition) => definition,
        Outcome::Source(source) => format!("= {}", source),
    }
}

#[cfg(unix)]
unsafe extern "C" {
    fn time(timer: *mut i64) -> i64;
    fn localtime(timer: *const i64) -> *mut i32;
}

// Local wall-clock time as hh:mm:ss
#[cfg(unix)]
pub fn now_hms() -> String {
    let mut seconds: i64 = 0;

    unsafe {
        time(&mut seconds);

        let tm = localtime(&seconds);

        if tm.is_null() {
            return "--:--:--".to_string();
        }

        // struct tm starts with tm_sec, tm_min, tm_hour as C ints on every ABI
        let second = *tm;
        let minute = *tm.add(1);
        let hour = *tm.add(2);

        format!("{:02}:{:02}:{:02}", hour, minute, second)
    }
}

#[cfg(windows)]
#[repr(C)]
struct SystemTime {
    year: u16,
    month: u16,
    day_of_week: u16,
    day: u16,
    hour: u16,
    minute: u16,
    second: u16,
    milliseconds: u16,
}

#[cfg(windows)]
unsafe extern "system" {
    fn GetLocalTime(system_time: *mut SystemTime);
}

// Local wall-clock time as hh:mm:ss
#[cfg(windows)]
pub fn now_hms() -> String {
    let mut now: SystemTime = unsafe { std::mem::zeroed() };

    unsafe { GetLocalTime(&mut now) };

    format!("{:02}:{:02}:{:02}", now.hour, now.minute, now.second)
}

// Local wall-clock time as hh:mm:ss
#[cfg(not(any(unix, windows)))]
pub fn now_hms() -> String {
    "--:--:--".to_string()
}
