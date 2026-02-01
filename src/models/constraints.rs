use chrono::{DateTime, Datelike, NaiveTime, Utc, Weekday};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TimeConstraint {
    pub start_time: Option<NaiveTime>,
    pub end_time: Option<NaiveTime>,
    pub allowed_days: Option<Vec<Weekday>>,
    pub excluded_dates: Vec<DateTime<Utc>>,
}

impl TimeConstraint {
    pub fn work_hours() -> Self {
        Self {
            start_time: NaiveTime::from_hms_opt(9, 0, 0),
            end_time: NaiveTime::from_hms_opt(18, 0, 0),
            allowed_days: Some(vec![
                Weekday::Mon,
                Weekday::Tue,
                Weekday::Wed,
                Weekday::Thu,
                Weekday::Fri,
            ]),
            excluded_dates: Vec::new(),
        }
    }

    pub fn is_time_allowed(&self, time: NaiveTime) -> bool {
        let after_start = self.start_time.is_none_or(|start| time >= start);
        let before_end = self.end_time.is_none_or(|end| time < end);
        after_start && before_end
    }

    pub fn is_day_allowed(&self, weekday: Weekday) -> bool {
        self.allowed_days
            .as_ref()
            .is_none_or(|days| days.contains(&weekday))
    }

    pub fn is_datetime_allowed(&self, dt: DateTime<Utc>) -> bool {
        if self
            .excluded_dates
            .iter()
            .any(|excluded| excluded.date_naive() == dt.date_naive())
        {
            return false;
        }

        self.is_day_allowed(dt.weekday()) && self.is_time_allowed(dt.time())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DependencyConstraint {
    pub depends_on: Vec<String>,
    pub blocks: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TaskConstraints {
    pub time: TimeConstraint,
    pub dependencies: DependencyConstraint,
    pub min_duration_minutes: Option<u32>,
    pub max_duration_minutes: Option<u32>,
    pub preferred_time_of_day: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_work_hours_constraint() {
        let constraint = TimeConstraint::work_hours();

        let work_time = NaiveTime::from_hms_opt(10, 30, 0).unwrap();
        assert!(constraint.is_time_allowed(work_time));

        let early_time = NaiveTime::from_hms_opt(7, 0, 0).unwrap();
        assert!(!constraint.is_time_allowed(early_time));

        assert!(constraint.is_day_allowed(Weekday::Mon));
        assert!(!constraint.is_day_allowed(Weekday::Sat));
    }
}
