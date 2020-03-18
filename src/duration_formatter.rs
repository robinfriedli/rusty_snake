use std::ops::Add;
use std::time::Duration;

pub trait DurationFormatter {
    fn format_duration(self) -> String;
}

impl DurationFormatter for Duration {
    fn format_duration(self) -> String {
        let total_seconds = self.as_secs();
        let seconds = total_seconds % 60;
        let minutes = total_seconds / 60;

        format!("{:02}:{:02}", minutes, seconds)
    }
}

impl DurationFormatter for Option<u64> {
    fn format_duration(self) -> String {
        match self {
            Some(time) => String::from(" (").add(Duration::from_millis(time).format_duration().as_str()).add(")"),
            None => String::from("")
        }
    }
}