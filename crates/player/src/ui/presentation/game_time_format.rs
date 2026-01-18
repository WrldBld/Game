use crate::application::dto::GameTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TimeOfDay {
    Morning,
    Afternoon,
    Evening,
    Night,
}

impl std::fmt::Display for TimeOfDay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let display_name = match self {
            TimeOfDay::Morning => "Morning",
            TimeOfDay::Afternoon => "Afternoon",
            TimeOfDay::Evening => "Evening",
            TimeOfDay::Night => "Night",
        };
        write!(f, "{display_name}")
    }
}

pub fn time_of_day(game_time: &GameTime) -> TimeOfDay {
    match game_time.hour {
        5..=11 => TimeOfDay::Morning,
        12..=17 => TimeOfDay::Afternoon,
        18..=21 => TimeOfDay::Evening,
        _ => TimeOfDay::Night,
    }
}

pub fn display_time(game_time: &GameTime) -> String {
    // Use server-provided formatted_time if available
    if let Some(ref formatted) = game_time.formatted_time {
        return formatted.clone();
    }

    // Fallback to local formatting
    let hour = game_time.hour;
    let minute = game_time.minute;

    let period = if hour >= 12 { "PM" } else { "AM" };
    let display_hour = if hour == 0 {
        12
    } else if hour > 12 {
        hour - 12
    } else {
        hour
    };

    format!("{}:{:02} {}", display_hour, minute, period)
}

pub fn display_date(game_time: &GameTime) -> String {
    // Use server-provided formatted_date if available
    if let Some(ref formatted) = game_time.formatted_date {
        return format!("{}, {}", formatted, display_time(game_time));
    }

    // Fallback to simple day display
    format!("Day {}, {}", game_time.day, display_time(game_time))
}
