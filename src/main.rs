extern crate glob;
extern crate ogg;
extern crate rand;
extern crate zip;

use glob::glob;
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

const CHANCE_FOR_DYNAMIC_PATTERN: i64 = 40;
const CUT_UP: i64 = 0;
const CUT_DOWN: i64 = 1;
const CUT_LEFT: i64 = 2;
const CUT_RIGHT: i64 = 3;
const CUT_NW: i64 = 4;
const CUT_NE: i64 = 5;
const CUT_SW: i64 = 6;
const CUT_SE: i64 = 7;
const CUT_OMNI: i64 = 8;
fn get_opposite_cut(direction: i64) -> i64 {
	let mut result = CUT_OMNI;
	if direction == CUT_UP {
		result = CUT_DOWN;
	} else if direction == CUT_DOWN {
		result = CUT_UP;
	} else if direction == CUT_RIGHT {
		result = CUT_LEFT;
	} else if direction == CUT_LEFT {
		result = CUT_RIGHT;
	} else if direction == CUT_NW {
		result = CUT_SE;
	} else if direction == CUT_NE {
		result = CUT_SW;
	} else if direction == CUT_SW {
		result = CUT_NE;
	} else if direction == CUT_SE {
		result = CUT_NW;
	}

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
	let duration_seconds: f64 = config_json._duration_seconds;
	let song_duration_beats: f64 = (duration_seconds / 60.0) * beats_per_minute;

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
fn get_random_x(note_type: i64) -> usize {
	let mut random_x = rand::thread_rng().gen_range(0, 4);
	if note_type == 0 {
		random_x = rand::thread_rng().gen_range(0, 2);
	} else if note_type == 1 {
		random_x = rand::thread_rng().gen_range(2, 4);
	}
	(random_x)
}
fn generate_dynamic_pattern() -> Pattern {
	//placeholder logic
	let mut notes = Vec::new();
	let mut obstacles = Vec::new();
	let description = "Dynamic pattern".to_owned();

	let mut prev_x: i64 = 4;
	let mut prev_y: i64 = 4;
	let mut prev_cut_direction = i64::from(rand::thread_rng().gen_range(0, 9));
	let mut cut_direction  = 8;
	let note_type = i64::from(rand::thread_rng().gen_range(0, 2));
	let number_of_notes = 2;//really 1
	for i in 0..number_of_notes {
		cut_direction = get_opposite_cut(prev_cut_direction);
		let mut x = get_random_x(note_type) as i64;
		let mut y = i64::from(rand::thread_rng().gen_range(0, 4));
		
		let description = format!("{},{}", y, x).to_owned();
		let beat_time_rel = i;
		let note = PatternNote {
			x: x as i64,
			y: y as i64,
			cut_direction,
			note_type,
			description,
			beat_time_rel,
		};

		notes.push(note);
		prev_x = x;
		prev_y = y;
		prev_cut_direction = cut_direction;
	}


	/*PatternNote {
		x: i64,
	y: i64,
			cut_direction: i64,
	note_type: i64,
			description: String,
	beat_time_rel: usize,
	}*/

	let pattern: Pattern = Pattern {
		description,
		notes,
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
		let mut beat_next_id = beat_id + pattern_note.beat_time_rel;
		if beat_next_id >= beats.len() {
			beat_next_id = beats.len() - 1;
		}
		let note_time_in_beats = (beats[beat_next_id].peak_time_sec / 60.0) * beats_per_minute;
		last_note_time_in_beats = note_time_in_beats;
		let mut pattern_x = pattern_note.x;
		if pattern_x < 0 {
			if note_type == 0 {
				pattern_x = rand::thread_rng().gen_range(0, 2);
			} else if note_type == 1 {
				pattern_x = rand::thread_rng().gen_range(2, 4);
			} else {
				pattern_x = rand::thread_rng().gen_range(0, 4);
			}
		}
		let mut pattern_y = pattern_note.y;
		if pattern_y < 0 {
			if note_type == 0 {
				pattern_y = rand::thread_rng().gen_range(0, 2);
			} else if note_type == 1 {
				pattern_y = rand::thread_rng().gen_range(2, 4);
			} else {
				pattern_y = rand::thread_rng().gen_range(0, 4);
			}
		}

		if pattern_x < 0 {
			pattern_x = 0;
		} else if pattern_x > 3 {
			pattern_x = 3;
		}
		if pattern_y < 0 {
			pattern_y = 0;
		} else if pattern_y > 3 {
			pattern_y = 3;
		}
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
#[allow(clippy::if_same_then_else)]
#[allow(clippy::cyclomatic_complexity)]
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

	//we'll treat the lowest 15% as nothing
	let lowest_threshold = lowest_pitch * 1.15;

	let hard_threshold = highest_pitch * 0.20;
	let normal_threshold = highest_pitch * 0.40;

	let mut beat_id = 0;
	for beat in beats {
		if beat.pitch > lowest_threshold && beat.peak_time_sec > pattern_end_time {
			let dynamic_pattern_chance = rand::thread_rng().gen_range(0, 100);

			if dynamic_pattern_chance <= CHANCE_FOR_DYNAMIC_PATTERN {
				let new_pattern = generate_dynamic_pattern();
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
