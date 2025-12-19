use chrono::{DateTime, Datelike, Duration, Timelike, Utc};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TimeOfDay {
    Morning,
    Afternoon,
    Evening,
    Night,
}

impl TimeOfDay {
    pub fn display_name(&self) -> &'static str {
        match self {
            TimeOfDay::Morning => "Morning",
            TimeOfDay::Afternoon => "Afternoon",
            TimeOfDay::Evening => "Evening",
            TimeOfDay::Night => "Night",
        }
    }
}

impl std::fmt::Display for TimeOfDay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GameTime {
    current: DateTime<Utc>,
    is_paused: bool,
}

impl Default for GameTime {
    fn default() -> Self {
        Self::new()
    }
}

impl GameTime {
    pub fn new() -> Self {
        Self {
            current: Utc::now(),
            is_paused: true,
        }
    }

    pub fn starting_at(start: DateTime<Utc>) -> Self {
        Self {
            current: start,
            is_paused: true,
        }
    }

    pub fn current(&self) -> DateTime<Utc> {
        self.current
    }

    pub fn set_time(&mut self, new_time: DateTime<Utc>) {
        self.current = new_time;
    }

    pub fn is_paused(&self) -> bool {
        self.is_paused
    }

    pub fn set_paused(&mut self, paused: bool) {
        self.is_paused = paused;
    }

    pub fn advance(&mut self, duration: Duration) {
        self.current = self.current + duration;
    }

    pub fn advance_hours(&mut self, hours: u32) {
        self.advance(Duration::hours(hours as i64));
    }

    pub fn advance_days(&mut self, days: u32) {
        self.advance(Duration::days(days as i64));
    }

    pub fn time_of_day(&self) -> TimeOfDay {
        let hour = self.current.hour();
        match hour {
            5..=11 => TimeOfDay::Morning,
            12..=17 => TimeOfDay::Afternoon,
            18..=21 => TimeOfDay::Evening,
            _ => TimeOfDay::Night,
        }
    }

    pub fn day_ordinal(&self) -> u32 {
        self.current.ordinal()
    }

    pub fn display_time(&self) -> String {
        let hour = self.current.hour();
        let minute = self.current.minute();

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

    pub fn display_date(&self) -> String {
        format!("Day {}, {}", self.day_ordinal(), self.display_time())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_game_time_is_paused() {
        let gt = GameTime::new();
        assert!(gt.is_paused());
    }

    #[test]
    fn time_of_day_mapping_is_standardized() {
        let morning = GameTime::starting_at(
            DateTime::parse_from_rfc3339("2024-01-01T05:00:00Z")
                .unwrap()
                .into(),
        );
        assert_eq!(morning.time_of_day(), TimeOfDay::Morning);

        let afternoon = GameTime::starting_at(
            DateTime::parse_from_rfc3339("2024-01-01T12:00:00Z")
                .unwrap()
                .into(),
        );
        assert_eq!(afternoon.time_of_day(), TimeOfDay::Afternoon);

        let evening = GameTime::starting_at(
            DateTime::parse_from_rfc3339("2024-01-01T18:00:00Z")
                .unwrap()
                .into(),
        );
        assert_eq!(evening.time_of_day(), TimeOfDay::Evening);

        let night = GameTime::starting_at(
            DateTime::parse_from_rfc3339("2024-01-01T03:00:00Z")
                .unwrap()
                .into(),
        );
        assert_eq!(night.time_of_day(), TimeOfDay::Night);
    }
}
