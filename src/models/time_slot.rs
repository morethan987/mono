use chrono::{DateTime, NaiveTime, Timelike, Utc, Weekday};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSlot {
    pub start: NaiveTime,
    pub end: NaiveTime,
    pub weekday: Option<Weekday>,
}

impl TimeSlot {
    pub fn new(start: NaiveTime, end: NaiveTime) -> Self {
        Self {
            start,
            end,
            weekday: None,
        }
    }

    pub fn with_weekday(mut self, weekday: Weekday) -> Self {
        self.weekday = Some(weekday);
        self
    }

    pub fn duration_minutes(&self) -> i64 {
        let start_mins = self.start.num_seconds_from_midnight() as i64 / 60;
        let end_mins = self.end.num_seconds_from_midnight() as i64 / 60;
        end_mins - start_mins
    }

    pub fn contains(&self, time: NaiveTime) -> bool {
        self.start <= time && time < self.end
    }

    pub fn overlaps(&self, other: &TimeSlot) -> bool {
        self.start < other.end && other.start < self.end
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TimeOfDay {
    Morning,
    Afternoon,
    Evening,
    Night,
}

impl TimeOfDay {
    pub fn from_hour(hour: u32) -> Self {
        match hour {
            6..=11 => TimeOfDay::Morning,
            12..=17 => TimeOfDay::Afternoon,
            18..=21 => TimeOfDay::Evening,
            _ => TimeOfDay::Night,
        }
    }

    pub fn from_datetime(dt: DateTime<Utc>) -> Self {
        Self::from_hour(dt.time().hour())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_slot_duration() {
        let slot = TimeSlot::new(
            NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
            NaiveTime::from_hms_opt(10, 30, 0).unwrap(),
        );
        assert_eq!(slot.duration_minutes(), 90);
    }

    #[test]
    fn test_time_slot_overlap() {
        let slot1 = TimeSlot::new(
            NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
            NaiveTime::from_hms_opt(11, 0, 0).unwrap(),
        );
        let slot2 = TimeSlot::new(
            NaiveTime::from_hms_opt(10, 0, 0).unwrap(),
            NaiveTime::from_hms_opt(12, 0, 0).unwrap(),
        );
        let slot3 = TimeSlot::new(
            NaiveTime::from_hms_opt(11, 0, 0).unwrap(),
            NaiveTime::from_hms_opt(12, 0, 0).unwrap(),
        );

        assert!(slot1.overlaps(&slot2));
        assert!(!slot1.overlaps(&slot3));
    }

    #[test]
    fn test_time_of_day() {
        assert_eq!(TimeOfDay::from_hour(8), TimeOfDay::Morning);
        assert_eq!(TimeOfDay::from_hour(14), TimeOfDay::Afternoon);
        assert_eq!(TimeOfDay::from_hour(19), TimeOfDay::Evening);
        assert_eq!(TimeOfDay::from_hour(23), TimeOfDay::Night);
    }
}
