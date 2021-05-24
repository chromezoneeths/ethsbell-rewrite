#![feature(decl_macro)]

use std::{
	env, fs,
	sync::{Arc, Mutex, RwLock},
};

use chrono::{DateTime, Local};
use schedule::{Schedule, ScheduleDefinition};

pub mod api;
mod frontend;
pub mod ical;
mod login;
pub mod schedule;

#[derive(Clone)]
struct SpecLock(Option<DateTime<Local>>);

#[macro_use]
extern crate rocket;
#[allow(dead_code)]
fn main() {
	// Build the schedule definition and do our first update.
	let schedule = {
		// Load the definition.
		let string = if cfg!(target_arch = "wasm32") {
			include_str!("../def.json").to_string()
		} else {
			fs::read_to_string(env::var("SCHEDULE_DEF").unwrap_or("./def.json".to_string()))
				.expect("Opened schedule definition")
		};

		// Deserialize the definition.
		let schedule_def: ScheduleDefinition =
			serde_json::from_str(&string).expect("Deserialized schedule definition");
		// Build the runtime schedule struct and run the first update.
		let schedule = Schedule::from(schedule_def);
		// Wrap the runtime schedule struct in a thread-safe container.
		Arc::new(RwLock::new(schedule))
	};
	let spec_lock = Arc::new(Mutex::new(SpecLock(None)));
	rocket::ignite()
		.attach(api::ApiFairing)
		.attach(frontend::FrontendFairing)
		.manage(schedule.clone())
		.manage(spec_lock)
		.launch();
}
