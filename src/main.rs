extern crate glob;
extern crate ogg;
extern crate rand;
extern crate zip;

use glob::glob;
use rand::distributions::{Distribution, Uniform};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Read;
use std::io::{Seek, Write};
use zip::result::ZipResult;
use zip::write::{FileOptions, ZipWriter};

const CHANCE_FOR_DYNAMIC_PATTERN: i64 = 50;
const MIN_BEAT_SPACING_TIME: f64 = 0.05;
struct CutDirection {
	cut_direction: i64,
	x_dir: i64,
	y_dir: i64,
	possible_transitions: [i64; 9],
}
const CUT_UP: CutDirection = CutDirection {
	cut_direction: 0,
	x_dir: 0,
	y_dir: 1,
	possible_transitions: [1, 1, 1, 1, 6, 6, 7, 7, 8],
};
const CUT_DOWN: CutDirection = CutDirection {
	cut_direction: 1,
	x_dir: 0,
	y_dir: -1,
	possible_transitions: [0, 0, 0, 0, 4, 4, 5, 5, 8],
};
const CUT_LEFT: CutDirection = CutDirection {
	cut_direction: 2,
	x_dir: -1,
	y_dir: 0,
	possible_transitions: [3, 3, 3, 3, 5, 5, 7, 7, 8],
};
const CUT_RIGHT: CutDirection = CutDirection {
	cut_direction: 3,
	x_dir: 1,
	y_dir: 0,
	possible_transitions: [2, 2, 2, 2, 4, 4, 6, 6, 8],
};
const CUT_NW: CutDirection = CutDirection {
	cut_direction: 4,
	x_dir: -1,
	y_dir: 1,
	possible_transitions: [7, 7, 7, 7, 7, 1, 1, 8, 8],
};
const CUT_NE: CutDirection = CutDirection {
	cut_direction: 5,
	x_dir: 1,
	y_dir: 1,
	possible_transitions: [6, 6, 6, 6, 6, 1, 1, 8, 8],
};
const CUT_SW: CutDirection = CutDirection {
	cut_direction: 6,
	x_dir: -1,
	y_dir: -1,
	possible_transitions: [5, 5, 5, 5, 5, 0, 0, 8, 8],
};
const CUT_SE: CutDirection = CutDirection {
	cut_direction: 7,
	x_dir: 1,
	y_dir: -1,
	possible_transitions: [4, 4, 4, 4, 4, 0, 0, 8, 8],
};
const CUT_OMNI: CutDirection = CutDirection {
	cut_direction: 8,
	x_dir: 0,
	y_dir: 0,
	possible_transitions: [0, 1, 2, 3, 4, 5, 6, 7, 8],
};
const CUT_DIRECTIONS: [CutDirection; 9] = [
	CUT_UP, CUT_DOWN, CUT_LEFT, CUT_RIGHT, CUT_NW, CUT_NE, CUT_SW, CUT_SE, CUT_OMNI,
];
fn get_next_cut(cut_direction: &CutDirection) -> &CutDirection {
	let random_transition_index = rand::thread_rng().gen_range(0, CUT_DIRECTIONS.len());
	let mut result: &CutDirection = &CUT_OMNI;
	//get a random cut direction from the possible transitions of the last CutDirection
	let direction = cut_direction.possible_transitions[random_transition_index];
	for cd in CUT_DIRECTIONS.iter() {
		if direction == cd.cut_direction {
			result = cd;
		}
	}
//	println!(
//		"{} to {}",
//		cut_direction.cut_direction, result.cut_direction
//	);
	(result)
}

fn main() -> std::io::Result<()> {
	println!("Start map creation");

	let map_string: String = create_bsaber_map();
	println!("Start zipping");
	//fs::remove_file("output/song.zip")?;
	//let output_folder = "/home/matt/Games/beatsaver-viewer-master/song.zip";
	let output_folder = "output/song.zip";
	let mut file = File::create(output_folder).expect("Couldn't create file");
	create_zip_archive(&mut file, map_string).expect("Couldn't create archive");
	println!("Finished zipping");

	Ok(())
}

#[derive(Serialize, Deserialize)]
#[allow(non_snake_case)] //they're not snake case in the json
struct JsonConfig {
	_version: String,
	_songName: String,
	_songSubName: String,
	_songAuthorName: String,
	_levelAuthorName: String,
	_beatsPerMinute: f64,
	_duration_seconds: f64,
	_songTimeOffset: f64,
	_shuffle: f64,
	_shufflePeriod: f64,
	_previewStartTime: f64,
	_previewDuration: f64,
	_songFilename: String,
	_coverImageFilename: String,
}

#[allow(non_snake_case)]
//0.3947392290249433 110 1182.4482
struct Beat {
	peak_time_sec: f64,
	id: usize,
	pitch: f64,
}

#[derive(Serialize, Deserialize)]
struct PatternCollection {
	easy_patterns: Vec<Pattern>,
	normal_patterns: Vec<Pattern>,
	hard_patterns: Vec<Pattern>,
}
#[derive(Serialize, Deserialize)]
struct Pattern {
	description: String,
	notes: Vec<PatternNote>,
	obstacles: Vec<PatternWall>,
}

#[derive(Serialize, Deserialize)]
struct PatternNote {
	x: i64,
	y: i64,
	cut_direction: i64,
	note_type: i64,
	description: String,
	beat_time_rel: usize,
}

#[derive(Serialize, Deserialize)]
struct PatternWall {
	x: i64,
	y: i64,
	duration: f64,
	padding: f64,
	width: f64,
	wall_type: i64,
	description: String,
	wall_time_rel: usize,
}

struct AddPatternResult {
	pattern_end_time: f64,
	contents: String,
	obstacles_content: String,
}

fn get_config_file() -> JsonConfig {
	let mut config_file = File::open("src/song/info.dat").unwrap();
	let mut config_data = String::new();
	config_file.read_to_string(&mut config_data).unwrap();
	let config_json: JsonConfig =
		serde_json::from_str(&config_data[..]).expect("Could not read config file");
	(config_json)
}

fn generate_patterns() -> PatternCollection {
	let mut easy_patterns = Vec::new();
	let mut normal_patterns = Vec::new();
	let mut hard_patterns = Vec::new();

	for entry in glob("src/patterns/easy/*.json").expect("Failed to read glob pattern") {
		match entry {
			Ok(path) => {
				let mut pattern_file = File::open(path).unwrap();
				let mut pattern_json = String::new();
				pattern_file.read_to_string(&mut pattern_json).unwrap();

				let pattern_notes: Pattern = serde_json::from_str(&pattern_json[..])
					.expect("Could not read pattern note json");
				//println!("pattern:{}",pattern_notes.description.to_owned());
				easy_patterns.push(pattern_notes);
			}
			Err(e) => println!("{:?}", e),
		}
	}
	for entry in glob("src/patterns/normal/*.json").expect("Failed to read glob pattern") {
		match entry {
			Ok(path) => {
				let mut pattern_file = File::open(path).unwrap();
				let mut pattern_json = String::new();
				pattern_file.read_to_string(&mut pattern_json).unwrap();

				let pattern_notes: Pattern = serde_json::from_str(&pattern_json[..])
					.expect("Could not read pattern note json");
				//println!("pattern:{}",pattern_notes.description.to_owned());
				normal_patterns.push(pattern_notes);
			}
			Err(e) => println!("{:?}", e),
		}
	}
	for entry in glob("src/patterns/hard/*.json").expect("Failed to read glob pattern") {
		match entry {
			Ok(path) => {
				let mut pattern_file = File::open(path).unwrap();
				let mut pattern_json = String::new();
				pattern_file.read_to_string(&mut pattern_json).unwrap();

				let pattern_notes: Pattern = serde_json::from_str(&pattern_json[..])
					.expect("Could not read pattern note json");
				//println!("pattern:{}",pattern_notes.description.to_owned());
				hard_patterns.push(pattern_notes);
			}
			Err(e) => println!("{:?}", e),
		}
	}
	let pattern_collection: PatternCollection = PatternCollection {
		easy_patterns,
		normal_patterns,
		hard_patterns,
	};

	(pattern_collection)
}
/*
*
*/
fn create_bsaber_map() -> String {
	fs::remove_file("src/song/ExpertPlus.dat").expect("Could not remove song json");
	let mut file =
		File::create("src/song/ExpertPlus.dat").expect("Could not create song json file");

	//read info.dat
	let config_json = get_config_file();

	let pattern_collection = generate_patterns();

	let version = "2.0.0";
	//get configs from info.dat
	let beats_per_minute: f64 = config_json._beatsPerMinute;
	//let duration_seconds: f64 = config_json._duration_seconds;
	//let song_duration_beats: f64 = (duration_seconds / 60.0) * beats_per_minute;

	//start of ExperPlus.json
	let mut contents: String = format!(
		"{{\"_version\":\"{}\",
			\"_BPMChanges\":[],
			\"_events\":[{{
				\"_time\":3.199899911880493,
				\"_type\":4,
				\"_value\":3
			}}],
			\"_notes\":[",
		version
	)
	.to_owned();

	//read the peak times and pitches
	let peak_times_file =
		File::open("src/song/peak_times.txt").expect("Could not read peak times file");
	let peak_times_buffer = BufReader::new(&peak_times_file);
	//we'll keep track of times for the peaks and where walls might go
	let mut beats = Vec::new();

	//find the highest and highest pitch while we're at it
	let mut highest_pitch: f64 = 0.0;
	let mut lowest_pitch: f64 = 1600.0;
	let mut prev_peak_time: f64 = 0.0;
	//iterate over the file line by line
	for line in peak_times_buffer.lines() {
		let line = line.unwrap();
		let mut iter = line.split_whitespace();
		//1.0216780045351475 175.67802
		//first number is the time (in seconds), second number is the pitch measurement
		let peak_time = iter.next().unwrap();
		let peak_id = iter.next().unwrap();
		let peak_pitch = iter.next().unwrap();
		//convert to floats
		let peak_time_as_float = peak_time.parse::<f64>().unwrap();
		let peak_id_as_usize = peak_id.parse::<usize>().unwrap();
		let peak_pitch_as_float = peak_pitch.parse::<f64>().unwrap();
		if peak_time_as_float - prev_peak_time >= MIN_BEAT_SPACING_TIME {
			let beat = Beat {
				peak_time_sec: peak_time_as_float,
				id: peak_id_as_usize,
				pitch: peak_pitch_as_float,
			};
			//store in vector
			beats.push(beat);
			//track highest pitch
			if peak_pitch_as_float > highest_pitch {
				highest_pitch = peak_pitch_as_float;
			}
			if peak_pitch_as_float < lowest_pitch {
				lowest_pitch = peak_pitch_as_float;
			}
		}
		prev_peak_time = peak_time_as_float;
	}

	let mut processed_notes = Vec::new();
	//generate the map
	contents = generate_map(
		contents,
		&beats,
		&pattern_collection,
		highest_pitch,
		lowest_pitch,
		beats_per_minute,
		&mut processed_notes,
	);
	//write it all out to file
	file.write_all(contents.as_bytes())
		.expect("Could not write song json to file");
	drop(file);
	(contents)
}
fn get_next_x(note_type:usize, direction: &CutDirection, prev_x: usize) -> usize {
	let mut x_dir: i64 = direction.x_dir;
	if x_dir == 0 {
		x_dir = rand::thread_rng().gen_range(-1, 1);
	}
	let random_change: i64 = x_dir * rand::thread_rng().gen_range(1, 3);

	let mut random_x: i64 = (prev_x as i64) + random_change;
	if note_type == 0 && random_x > 1 {
		random_x = 1;
	}else if note_type == 1 && random_x < 2 {
		random_x = 2;
	}
	if random_x < 0 {
		random_x = 0;
	} else if random_x > 3 {
		random_x = 3;
	}

	(random_x as usize)
}
fn get_next_y(direction: &CutDirection, prev_y: usize) -> usize {
	let mut y_dir: i64 = direction.y_dir;
	if y_dir == 0 {
		y_dir = rand::thread_rng().gen_range(-1, 1);
	}
	let random_change: i64 = y_dir * rand::thread_rng().gen_range(1, 3);

	let mut random_y: i64 = (prev_y as i64) + random_change;
	if random_y < 0 {
		random_y = 0;
	} else if random_y > 2 {
		random_y = 2;
	}

	(random_y as usize)
}

#[allow(clippy::cyclomatic_complexity)]
fn generate_dynamic_pattern(
	left_prev_cut_direction: &CutDirection,
	left_prev_x: usize,
	left_prev_y: usize,
	right_prev_cut_direction: &CutDirection,
	right_prev_x: usize,
	right_prev_y: usize,
) -> Pattern {
	//placeholder logic
	let mut notes: Vec<PatternNote> = Vec::new();
	let mut obstacles = Vec::new();
	let description = "Dynamic pattern".to_owned();

	//let center = rand::thread_rng().gen_range(1, 3); //X=1 or 2

	let number_of_notes = rand::thread_rng().gen_range(4, 9);

	//LEFT

	let mut l_prev_cut_direction = left_prev_cut_direction;
	let mut l_prev_x = left_prev_x;
	let mut l_prev_y = left_prev_y;
	
	
	let mut r_prev_cut_direction = right_prev_cut_direction;
	let mut r_prev_x = right_prev_x;
	let mut r_prev_y = right_prev_y;

	let mut left_time_rel = 0;
	let mut right_time_rel = 0;
	for i in 0..number_of_notes {
		//let direction: CutDirection = get_cut_direction(requested_x, requested_y, prev_cut_direction);
		let direction: &CutDirection = get_next_cut(l_prev_cut_direction);

		let requested_x = get_next_x(0, direction, l_prev_x);
		let requested_y = get_next_y(direction, l_prev_y);

		let description = format!("{},{}", requested_x, requested_y).to_owned();

		let note = PatternNote {
			x: requested_x as i64,
			y: requested_y as i64,
			cut_direction: direction.cut_direction,
			note_type: 0,
			description,
			beat_time_rel: left_time_rel,
		};

		notes.push(note);
		l_prev_x = requested_x;
		l_prev_y = requested_y;
		l_prev_cut_direction = direction;
		left_time_rel += 1;

		//RIGHT

		//let direction: CutDirection = get_cut_direction(requested_x, requested_y, prev_cut_direction);
		let direction: &CutDirection = get_next_cut(r_prev_cut_direction);

		let requested_x = get_next_x(1, direction, r_prev_x);
		let requested_y = get_next_y(direction, r_prev_y);

		let description = format!("{},{}", requested_x, requested_y).to_owned();

		let note = PatternNote {
			x: requested_x as i64,
			y: requested_y as i64,
			cut_direction: direction.cut_direction,
			note_type: 1,
			description,
			beat_time_rel: right_time_rel,
		};

		notes.push(note);
		r_prev_x = requested_x;
		r_prev_y = requested_y;
		r_prev_cut_direction = direction;
		right_time_rel += 1;
	}

	//compatibility checks
	let mut l_x = -1;
	let mut l_y = -1;
	let mut l_d = 8;
	let mut l_t = 0;
	let mut r_x = -1;
	let mut r_y = -1;
	let mut r_d = 8;
	let mut r_t = 0;
	let mut compatible_notes : Vec<PatternNote> = Vec::new();
	for note in &mut notes {
		if note.note_type == 0 {
			l_x = note.x;
			l_y = note.y;
			l_d = note.cut_direction;
			l_t = note.beat_time_rel;
		} else if note.note_type == 1 {
			r_x = note.x;
			r_y = note.y;
			r_d = note.cut_direction;
			r_t = note.beat_time_rel;
		}
		let mut occupy_same_coords = l_x == r_x && l_y == r_y && l_t == r_t;
		while occupy_same_coords {
			println!("Conflict at {},{} vs {},{}",l_x,l_y,r_x,r_y);
			let cut_direction: &CutDirection = &CUT_DIRECTIONS[r_d as usize];
			let mut x_dir: i64 = cut_direction.x_dir;
			if x_dir == 0 {
				x_dir = rand::thread_rng().gen_range(-1, 1);
			}
			let random_x_change: i64 = x_dir * rand::thread_rng().gen_range(1, 3);
			let mut new_x = r_x + random_x_change;
			
			let mut y_dir: i64 = cut_direction.y_dir;
			if y_dir == 0 {
				y_dir = rand::thread_rng().gen_range(-1, 1);
			}
			let random_y_change: i64 = y_dir * rand::thread_rng().gen_range(1, 3);
			let mut new_y = r_y + random_y_change;
			//if they go off the edge then have them wrap around
			if new_x < 0 {
				new_x = 3;
			} else if new_x > 3 {
				new_x = 0;
			}
			if new_y < 0 {
				new_y = 2;
			} else if new_y > 2 {
				new_y = 0;
			}
			println!("Conflict resolved right saber to {},{} -> {},{}",r_x,r_y,new_x,new_y);
			r_x = new_x;
			r_y = new_y;
			note.x = new_x;
			note.y = new_y;
			
			
			occupy_same_coords = l_x == r_x && l_y == r_y && l_t == r_t;
		}
		if note.note_type == 0 {
			
			//println!("Left Note at {},{} d={} t={}",l_x,l_y,l_d,l_t);
			let new_note = PatternNote {
				x: l_x,
				y: l_y,
				cut_direction: l_d,
				note_type: 0,
				description:note.description.to_owned(),
				beat_time_rel:l_t,
			};
	
			compatible_notes.push(new_note);
			
		} else if note.note_type == 1 {
			//println!("Right Note at {},{} d={} t={}",r_x,r_y,r_d,r_t);
			let new_note = PatternNote {
				x: r_x,
				y: r_y,
				cut_direction: r_d,
				note_type: 1,
				description:note.description.to_owned(),
				beat_time_rel:r_t,
			};
	
			compatible_notes.push(new_note);
			l_x = -1;
			l_y = -1;
			l_d = 8;
			l_t = 0;
			r_x = -1;
			r_y = -1;
			r_d = 8;
			r_t = 0;
		}
			

	}

	let pattern: Pattern = Pattern {
		description,
		notes:compatible_notes,
		obstacles,
	};

	(pattern)
}

#[allow(clippy::too_many_arguments)]
fn add_pattern(
	pattern: &Pattern,
	mut contents: String,
	beats: &[Beat],
	processed_notes: &mut Vec<PatternNote>,
	beat_id: usize,
	beats_per_minute: f64,
	mut pattern_end_time: f64,
	mut obstacles_content: String,
) -> AddPatternResult {
	println!("pattern:{}", pattern.description.to_owned());
	let mut last_note_time_in_beats = 0.0;

	let note_type = rand::thread_rng().gen_range(0, 1);
	for pattern_note in &pattern.notes {
		let mut beat_next_id = beat_id + pattern_note.beat_time_rel + 1;
		if beat_next_id >= beats.len() {
			beat_next_id = beats.len() - 1;
		}
		let note_time_in_beats = (beats[beat_next_id].peak_time_sec / 60.0) * beats_per_minute;
		last_note_time_in_beats = note_time_in_beats;
		let pattern_x = pattern_note.x;
		let pattern_y = pattern_note.y;

		let mut pattern_note_type = pattern_note.note_type;
		if pattern_note_type < 0 {
			pattern_note_type = note_type;
		}
		let pattern_cut_direction = pattern_note.cut_direction;

		pattern_end_time = beats[beat_next_id].peak_time_sec;
		//create the note using the values we calculated
		let note: String = create_note_json(
			beat_id,
			pattern_end_time,
			1600.0,
			note_time_in_beats,
			pattern_x,
			pattern_y,
			pattern_note_type,
			pattern_cut_direction,
			pattern_note.description.to_owned(),
		);
		//add the note pattern to the json string
		contents.push_str(&note);
		let processed_note = PatternNote {
			x: pattern_note.x,
			y: pattern_note.y,
			cut_direction: pattern_note.cut_direction,
			note_type: pattern_note.note_type,
			description: "Test".to_owned(),
			beat_time_rel: pattern_note.beat_time_rel,
		};
		processed_notes.push(processed_note);
	}

	for wall in &pattern.obstacles {
		let mut beat_next_id = beat_id + wall.wall_time_rel;
		if beat_next_id >= beats.len() {
			beat_next_id = beats.len() - 1;
		}
		let wall_time_in_beats = (beats[beat_next_id].peak_time_sec / 60.0) * beats_per_minute;

		let mut wall_duration = wall.duration;
		if wall_duration < 0.0 {
			wall_duration = last_note_time_in_beats - wall_time_in_beats;
		}
		if wall_duration - wall.padding > 0.0 {
			let wall_json = format!(
				"{{
					\"_time\": {},
					\"_lineIndex\": {},
					\"_lineLayer\": {},
					\"_type\": {},
					\"_duration\": {},
					\"_width\": {}
				}},",
				wall_time_in_beats + wall.padding,
				wall.x,
				wall.y,
				wall.wall_type,
				wall_duration - wall.padding,
				wall.width
			);
			obstacles_content.push_str(&wall_json);
		}
	}
	let results: AddPatternResult = AddPatternResult {
		pattern_end_time,
		contents,
		obstacles_content,
	};
	(results)
}
/*
* Generate a bsaber map using the information provided
*/
fn generate_map(
	mut contents: String,
	beats: &[Beat],
	pattern_collection: &PatternCollection,
	highest_pitch: f64,
	lowest_pitch: f64,
	beats_per_minute: f64,
	processed_notes: &mut Vec<PatternNote>,
) -> String {
	let mut obstacles_content: String = " ".to_owned();

	let mut pattern_map = HashMap::new();
	let mut pattern_end_time: f64 = 0.0;

	//we'll treat the lowest 10% as nothing
	let lowest_threshold = lowest_pitch * 1.10;

	let hard_threshold = highest_pitch * 0.25;
	let normal_threshold = highest_pitch * 0.65;

	let mut beat_id = 0;

	let mut left_prev_cut_direction: &CutDirection = &CUT_OMNI;
	let mut left_prev_x = 1;
	let mut left_prev_y = 2;
	let mut right_prev_cut_direction: &CutDirection = &CUT_OMNI;
	let mut right_prev_x = 2;
	let mut right_prev_y = 2;

	for beat in beats {
		//println!("peak :{} vs {}",beat.peak_time_sec,pattern_end_time);
		if beat.pitch > lowest_threshold && beat.peak_time_sec > pattern_end_time {
			let dynamic_pattern_chance = rand::thread_rng().gen_range(0, 100);

			if dynamic_pattern_chance <= CHANCE_FOR_DYNAMIC_PATTERN {
				let new_pattern: Pattern = generate_dynamic_pattern(
					left_prev_cut_direction,
					left_prev_x,
					left_prev_y,
					right_prev_cut_direction,
					right_prev_x,
					right_prev_y,
				);

				for pattern_note in &new_pattern.notes {
					if pattern_note.note_type == 0 {
						left_prev_cut_direction =
							&CUT_DIRECTIONS[pattern_note.cut_direction as usize];
						left_prev_x = pattern_note.x as usize;
						left_prev_y = pattern_note.y as usize;
					} else if pattern_note.note_type == 1 {
						right_prev_cut_direction =
							&CUT_DIRECTIONS[pattern_note.cut_direction as usize];
						right_prev_x = pattern_note.x as usize;
						right_prev_y = pattern_note.y as usize;
					}
				}
				let add_pattern_results: AddPatternResult = add_pattern(
					&new_pattern,
					contents,
					beats,
					processed_notes,
					beat_id,
					beats_per_minute,
					pattern_end_time,
					obstacles_content,
				);
				contents = add_pattern_results.contents;
				pattern_end_time = add_pattern_results.pattern_end_time;
				obstacles_content = add_pattern_results.obstacles_content;
			} else {
				let mut patterns = &pattern_collection.easy_patterns;
				if beat.pitch <= hard_threshold {
					patterns = &pattern_collection.normal_patterns;
				} else if beat.pitch <= normal_threshold {
					patterns = &pattern_collection.hard_patterns;
				}
				let random_pattern_id = rand::thread_rng().gen_range(0, patterns.len());

				//x = rand::thread_rng().gen_range(0, 2);
				let random_pattern = &patterns[random_pattern_id];

				let pattern = pattern_map.entry(beat.id).or_insert(random_pattern);

				let add_pattern_results: AddPatternResult = add_pattern(
					pattern,
					contents,
					beats,
					processed_notes,
					beat_id,
					beats_per_minute,
					pattern_end_time,
					obstacles_content,
				);
				contents = add_pattern_results.contents;
				pattern_end_time = add_pattern_results.pattern_end_time;
				obstacles_content = add_pattern_results.obstacles_content;
			}
		}
		beat_id += 1;
	}

	//generate walls
	//contents = generate_walls(contents, wall_times, &peak_pitches, beats_per_minute, time, lowest_pitch);

	let contents_end: String = format!(
		"{{
			\"_time\": 0.0,
			\"_lineIndex\": 3,
			\"_lineLayer\": 3,
			\"_type\": 1,
			\"_cutDirection\": 0
		}}],
		\"_obstacles\": [{}
			{{
				\"_time\": 0,
				\"_lineIndex\": 0,
				\"_type\": 0,
				\"_duration\": 1,
				\"_width\": 1
			}}]
		}}",
		obstacles_content
	)
	.to_owned();

	//add to json string
	contents.push_str(&contents_end);
	let note_count = processed_notes.len();
	println!("note count:{}", note_count.to_owned());
	//return the completed json string
	(contents)
}
/*
	* Creates a note json string based on the date provided
	*/
#[allow(clippy::too_many_arguments)]
fn create_note_json(
	id: usize,
	peak_time: f64,
	peak_pitch: f64,
	time_beats: f64,
	x: i64,
	y: i64,
	note_type: i64,
	cut_direction: i64,
	description: String,
) -> String {
	let note: String = format!(
		"{{
			\"_x\": {},
			\"_real_time\": {},
			\"_pitch\": {},
			\"_time\": {},
			\"_lineIndex\": {},
			\"_lineLayer\": {},
			\"_type\": {},
			\"_cutDirection\": {},
			\"description\":\"{}\"
		}},",
		id, peak_time, peak_pitch, time_beats, x, y, note_type, cut_direction, description
	)
	.to_owned();
	(note)
}

/*
	* Zip up the important files from /src/song
	* This is the zip beat saber, bsaber, or the map previewer can use
 */
fn create_zip_archive<T: Seek + Write>(buf: &mut T, song_json: String) -> ZipResult<()> {
	let mut writer = ZipWriter::new(buf);
	println!("Adding ExpertPlus");

	writer.start_file("ExpertPlus.dat", FileOptions::default())?;
	writer.write_all(song_json.as_bytes())?;

	println!("Adding cover");
	let cover = include_bytes!("./song/cover.jpg");
	writer.start_file("cover.jpg", FileOptions::default())?;
	writer.write_all(cover)?;
	println!("Adding info");
	let info = include_bytes!("./song/info.dat");
	writer.start_file("info.dat", FileOptions::default())?;
	writer.write_all(info)?;
	println!("Adding song");
	let song = include_bytes!("./song/song.ogg");
	writer.start_file("song.ogg", FileOptions::default())?;
	writer.write_all(song)?;
	let song_egg = include_bytes!("./song/song.egg");
	writer.start_file("song.egg", FileOptions::default())?;
	writer.write_all(song_egg)?;
	println!("Adding done");
	writer.finish()?;
	Ok(())
}
