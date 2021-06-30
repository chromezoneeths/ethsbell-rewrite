//! Everything relating to our schedule structures.
use std::{collections::HashMap, convert::TryInto};

use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, NaiveDateTime, NaiveTime, Timelike};
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::ical::IcalEvent;

/// The definition of the schedule.
/// ```rust
/// use ethsbell_rewrite::schedule::ScheduleDefinition;
/// use chrono::naive::NaiveTime;
/// use std::collections::HashMap;
/// let schedule_text = "{\"calendar_url\":\"http://example.com/cal.ical\", \"override_calendar_url\":\"http://example.com/cal.ical\", \"schedule_types\": {}, \"typical_schedule\": []}";
/// let schedule: ScheduleDefinition = serde_json::from_str(&schedule_text).unwrap();
/// ```
#[cfg_attr(feature = "ws", derive(JsonSchema))]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ScheduleDefinition {
	/// The URL of the ical calendar we fetch the schedule's data from.
	pub calendar_url: Option<String>,
	/// The URL of the ical calendar we fetch any overrides from.
	pub override_calendar_url: Option<String>,
	/// All of the types of schedule there are.
	pub schedule_types: HashMap<String, ScheduleType>,
	/// The typical schedule.
	/// Should have seven elements, starting on Sunday and ending on Saturday.
	pub typical_schedule: Vec<String>,
}

/// A type of schedule that can occur.
/// ```rust
/// use ethsbell_rewrite::schedule::ScheduleType;
/// use chrono::naive::NaiveTime;
/// let schedule_text = "{\"friendly_name\":\"Test Schedule\", \"periods\": []}";
/// let schedule: ScheduleType = serde_json::from_str(&schedule_text).unwrap();
/// ```
#[cfg_attr(feature = "ws", derive(JsonSchema))]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ScheduleType {
	/// A human-friendly name for the schedule, like "No School" or "E-Learning: Blue Day"
	pub friendly_name: String,
	/// A list of the periods in the day.
	pub periods: Vec<Period>,
	/// The regular expression which calendar entries for this schedule match
	#[cfg_attr(feature = "ws", schemars(skip))]
	#[serde(with = "serde_regex")]
	pub regex: Option<Regex>,
}
impl ScheduleType {
	pub fn at_time(&self, time: NaiveTime) -> [Option<Period>; 3] {
		if self.periods.len() == 0 {
			[None, None, None]
		} else {
			let mut before: Option<Period> = None;
			let mut current: Option<Period> = None;
			let mut next: Option<Period> = None;
			self.periods.iter().for_each(|period| {
				if period.end < time {
					match before.clone() {
						Some(before_) if before_.end < period.end => before = Some(period.clone()),
						None => before = Some(period.clone()),
						_ => {}
					}
				} else if period.start > time {
					match next.clone() {
						Some(next_) if next_.start > period.start => next = Some(period.clone()),
						None => next = Some(period.clone()),
						_ => {}
					};
				} else {
					current = Some(period.clone())
				}
			});
			match (&before, &current, &next) {
				(Some(before), None, Some(next)) => {
					current = Some(Period {
						friendly_name: "Passing".to_string(),
						start: before.end,
						end: next.start,
						start_timestamp: 0,
						end_timestamp: 0,
						kind: PeriodType::Passing,
					})
				}
				(None, None, Some(next)) => {
					current = Some(Period {
						friendly_name: "Before school".to_string(),
						start: NaiveTime::from_hms(0, 0, 0),
						end: next.start,
						start_timestamp: 0,
						end_timestamp: 0,
						kind: PeriodType::BeforeSchool,
					})
				}
				(Some(before), None, None) => {
					current = Some(Period {
						friendly_name: "After School".to_string(),
						start: before.end,
						end: NaiveTime::from_hms(23, 59, 59),
						start_timestamp: 0,
						end_timestamp: 0,
						kind: PeriodType::AfterSchool,
					})
				}
				_ => {}
			}
			[before, current, next]
		}
	}
	pub fn first_class(&self) -> Option<Period> {
		self.periods
			.iter()
			.filter(|v| match v.kind {
				PeriodType::Class(_) => true,
				_ => false,
			})
			.map(|v| v.clone())
			.next()
	}
}

/// The definition for a period.
/// ```rust
/// use ethsbell_rewrite::schedule::{Period, PeriodType};
/// use chrono::naive::NaiveTime;
/// let period_text = "{\"friendly_name\":\"Test Period\", \"start\":\"08:00:00\", \"end\":\"09:00:00\",\"kind\":{\"Class\": 0}}";
/// let period: Period = serde_json::from_str(&period_text).unwrap();
/// assert_eq!(period, Period {
/// 	friendly_name: "Test Period".to_string(),
/// 	start: NaiveTime::from_hms(8,0,0),
///		end: NaiveTime::from_hms(9,0,0),
///		kind: PeriodType::Class(0)
/// })
/// ```
#[cfg_attr(feature = "ws", derive(JsonSchema))]
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Period {
	/// A human-friendly name for this period, like "First Period".
	pub friendly_name: String,
	/// The start of this period.
	pub start: NaiveTime,
	#[serde(skip_deserializing)]
	pub start_timestamp: u64,
	/// The end of this period.
	pub end: NaiveTime,
	#[serde(skip_deserializing)]
	pub end_timestamp: u64,
	/// The type of this period.
	pub kind: PeriodType,
}
impl Period {
	pub fn populate(&mut self, date: DateTime<Local>) {
		let start_date = date.clone();
		let end_date = date.clone();
		self.start_timestamp = start_date
			.with_hour(self.start.hour())
			.unwrap()
			.with_minute(self.start.minute())
			.unwrap()
			.with_second(self.start.second())
			.unwrap()
			.timestamp() as u64;
		self.end_timestamp = end_date
			.with_hour(self.end.hour())
			.unwrap()
			.with_minute(self.end.minute())
			.unwrap()
			.with_second(self.end.second())
			.unwrap()
			.timestamp() as u64;
	}
}

/// The types a period can be.
#[cfg_attr(feature = "ws", derive(JsonSchema))]
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub enum PeriodType {
	/// This period has a class in it, and it is this index in a student's schedule.
	Class(usize),
	/// This period is always lunch.
	Lunch,
	/// This period is always a break.
	Break,
	/// This period is AM support.
	AMSupport,
	/// This period is always a passing period.
	/// Declaring this is optional; frontend applications should automatically report a passing period when between two existing periods.
	Passing,
	/// This period is before school.
	BeforeSchool,
	/// This period is after school.
	AfterSchool,
}

#[cfg_attr(feature = "ws", derive(JsonSchema))]
#[derive(Serialize, Clone)]
pub struct Schedule {
	pub last_updated: NaiveDateTime,
	pub calendar: HashMap<NaiveDate, Vec<Event>>,
	pub definition: ScheduleDefinition,
}
#[cfg(feature = "pull")]
impl From<ScheduleDefinition> for Schedule {
	fn from(def: ScheduleDefinition) -> Self {
		let mut new = Schedule {
			last_updated: NaiveDateTime::from_timestamp(0, 0),
			calendar: HashMap::new(),
			definition: def,
		};
		new.update();
		new
	}
}
impl Schedule {
	#[cfg(feature = "pull")]
	pub fn update(&mut self) {
		// Fetch the primary calendar
		println!("Fetching main calendar...");
		let calendar_data = match &self.definition.calendar_url {
			Some(url) => IcalEvent::get(&url),
			None => vec![],
		};
		// Fetch the override calendar
		println!("Fetching override calendar...");
		let override_calendar_data = match &self.definition.override_calendar_url {
			Some(url) => IcalEvent::get(&url),
			None => vec![],
		};
		// Apply the primary calendar
		ical_to_ours(self, &calendar_data);
		// Apply the override calendar
		ical_to_ours(self, &override_calendar_data);
		// Update the last-updated value
		self.last_updated = Local::now().naive_local();
	}
	pub fn is_update_needed(&self) -> bool {
		self.last_updated.date() != Local::now().date().naive_local()
	}
	pub fn on_date(&self, date: NaiveDate) -> ScheduleType {
		let mut literal: Option<ScheduleType> = None;
		let special: Option<String> = self
			.calendar
			.iter()
			.filter(|v| v.0 == &date)
			.map(|v| {
				for i in v.1 {
					match i {
						Event::ScheduleOverride(s) => {
							return Some(s);
						}
						Event::ScheduleLiteral(s) => {
							literal = Some(serde_json::from_str(&s).unwrap());
							return None;
						}
						_ => {}
					}
				}
				None
			})
			.filter(|v| v.is_some())
			.map(|v| v.unwrap().clone())
			.next();
		match special {
			Some(name) => self.definition.schedule_types.get(&name).unwrap().clone(),
			None => match literal {
				Some(schedule) => schedule,
				None => {
					let weekday: usize = date.weekday().num_days_from_sunday().try_into().unwrap();
					let name = self.definition.typical_schedule[weekday].clone();
					self.definition.schedule_types.get(&name).unwrap().clone()
				}
			},
		}
	}
}

/// Types of calendar events.
#[cfg_attr(feature = "ws", derive(JsonSchema))]
#[derive(Serialize, Clone, PartialEq)]
pub enum Event {
	/// This variant causes an override of the current schedule to the schedule named in the variant.
	ScheduleOverride(String),
	/// This variant causes the schedule contained within to be used
	ScheduleLiteral(String),
	/// This variant causes a special event message to be included in the API response.
	SpecialEvent(String),
}

/// Write a Vec<IcalEvent> to our runtime schedule struct.
pub fn ical_to_ours(schedule: &mut Schedule, data: &Vec<IcalEvent>) {
	// For every ical event...
	data.iter().for_each(|event| {
		let start = event.start.unwrap();
		// The end is either 1 day after the start or equal to the defined end.
		let mut end = event.end.unwrap_or(start + Duration::days(1));
		// If the defined end is on the same day, we'll pretend it's the next day.
		if end == start {
			end += Duration::days(1)
		}
		// Start on the starting date, of course...
		let mut day = start.clone();
		while day < end {
			// Get the calendar's response for the day, whether or not it exists.
			let date = schedule.calendar.get(&day);
			// Create it if it doesn't exist.
			match &date {
				Some(_) => {}
				None => {
					schedule.calendar.insert(day, vec![]);
				}
			}
			// Unwrap the calendar's entry, now that we know it exists.
			let date = schedule.calendar.get_mut(&day).unwrap();
			// Check if the summary is a literal schedule
			let literal_header = "LITERAL SCHEDULE ";
			if event
				.description
				.as_ref()
				.unwrap_or(&"".to_string())
				.starts_with(literal_header)
			{
				let json = event
					.description
					.as_ref()
					.unwrap()
					.to_string()
					.chars()
					.skip(literal_header.len())
					.collect::<String>();
				let result = serde_json::from_str::<ScheduleType>(&json);
				if result.is_ok() {
					date.push(Event::ScheduleLiteral(json));
					return;
				} else {
					println!("Error parsing schedule literal: {:?}", result.unwrap_err())
				}
			}
			// Check against every schedule
			let mut is_schedule_event = false;
			for i in &schedule.definition.schedule_types {
				// If this schedule's regex matches...
				if i.1.regex.is_some()
					&& i.1
						.regex
						.as_ref()
						.unwrap()
						.is_match(&event.summary.as_ref().unwrap())
				{
					let mut found = false;
					// Check to see if a special schedule already exists for today...
					for o in date.iter_mut() {
						match o {
							// If it does, replace it with the new schedule.
							Event::ScheduleOverride(schedule) => {
								*schedule = i.0.clone();
								found = true;
								is_schedule_event = true
							}
							_ => {}
						}
					}
					if !found {
						// Otherwise, create a new event entry.
						date.push(Event::ScheduleOverride(i.0.clone()));
						is_schedule_event = true;
					}
				}
			}
			if !is_schedule_event {
				// If this event didn't match any special schedules, add it as a non-schedule Special Event.
				let new_event = Event::SpecialEvent(event.summary.as_ref().unwrap().clone());
				if !date.contains(&new_event) {
					date.push(new_event)
				}
			}
			// Move to the next day in the event.
			day += Duration::days(1)
		}
	})
}
