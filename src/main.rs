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

const CUT_UP: i64 = 0;
const CUT_DOWN: i64 = 1;
const CUT_LEFT: i64 = 2;
const CUT_RIGHT: i64 = 3;
const CUT_NW: i64 = 4;
const CUT_NE: i64 = 5;
const CUT_SW: i64 = 6;
const CUT_SE: i64 = 7;
const CUT_OMNI: i64 = 8;

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
	beatsPerMinute: f64,
	beatsPerBar: f64,
	noteJumpSpeed: f64,
	shuffle: f64,
	shufflePeriod: f64,
	duration_seconds: f64,
}

#[allow(non_snake_case)]
//0.3947392290249433 110 1182.4482
struct Beat {
	peak_time_sec: f64,
	id: usize,
	pitch: f64,
}

#[derive(Serialize, Deserialize)]
struct PatternNotes {
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

fn get_config_file() -> JsonConfig {
	let mut config_file = File::open("src/song/info.json").unwrap();
	let mut config_data = String::new();
	config_file.read_to_string(&mut config_data).unwrap();
	let config_json: JsonConfig =
		serde_json::from_str(&config_data[..]).expect("Could not read config file");
	(config_json)
}

fn generate_patterns() -> Vec<PatternNotes> {
	let mut patterns = Vec::new();
	for entry in glob("src/patterns/*.json").expect("Failed to read glob pattern") {
		match entry {
			Ok(path) => {
				let mut pattern_file = File::open(path).unwrap();
				let mut pattern_json = String::new();
				pattern_file.read_to_string(&mut pattern_json).unwrap();

				let pattern_notes: PatternNotes = serde_json::from_str(&pattern_json[..])
					.expect("Could not read pattern note json");
				//println!("pattern:{}",pattern_notes.description.to_owned());
				patterns.push(pattern_notes);
			}
			Err(e) => println!("{:?}", e),
		}
	}

	(patterns)
}
/*
*
*/
fn create_bsaber_map() -> String {
	fs::remove_file("src/song/ExpertPlus.json").expect("Could not remove song json");
	let mut file =
		File::create("src/song/ExpertPlus.json").expect("Could not create song json file");

	//read info.json
	let config_json = get_config_file();

	let patterns = generate_patterns();

	let version = "1.0.0";
	//get configs from info.json
	let beats_per_minute: f64 = config_json.beatsPerMinute;
	let beats_per_bar = config_json.beatsPerBar;
	let note_jump_speed = config_json.noteJumpSpeed;
	let shuffle = config_json.shuffle;
	let shuffle_period = config_json.shufflePeriod;
	let song_duration_beats: f64 = (config_json.duration_seconds / 60.0) * beats_per_minute;

	//start of ExperPlus.json
	let mut contents: String = format!(
		"{{\"_version\": \"{}\",
			\"_beatsPerMinute\": {},
			\"_beatsPerBar\": {},
			\"_noteJumpSpeed\": {},
			\"_shuffle\": {},
			\"_shufflePeriod\": {},
			\"_time\": {},
			\"_suggestions\":[\"Custom Base Colors\"],
			\"_colorRight\":{{\"r\":0.35,\"g\":0.05,\"b\":0.54}},
			\"_colorLeft\":{{\"r\":0.54,\"g\":0.42,\"b\":0.050}},
			\"_songTimeOffset\": 0.0,
			\"_notes\": [",
		version,
		beats_per_minute,
		beats_per_bar,
		note_jump_speed,
		shuffle,
		shuffle_period,
		song_duration_beats
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
		&patterns,
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
/*
* Generate a bsaber map using the information provided
*/
#[allow(clippy::if_same_then_else)]
#[allow(clippy::cyclomatic_complexity)]
fn generate_map(
	mut contents: String,
	beats: &[Beat],
	patterns: &[PatternNotes],
	highest_pitch: f64,
	lowest_pitch: f64,
	beats_per_minute: f64,
	processed_notes: &mut Vec<PatternNote>,
) -> String {
	let mut obstacles_content: String = " ".to_owned();

	let mut pattern_map = HashMap::new();
	let mut pattern_end_time: f64 = 0.0;
	//iterate over the peak times

	let mut beat_id = 0;
	for beat in beats {
		if beat.pitch > lowest_pitch && beat.peak_time_sec > pattern_end_time {
			let random_pattern_id = rand::thread_rng().gen_range(0, patterns.len());
			//x = rand::thread_rng().gen_range(0, 2);
			let random_pattern = &patterns[random_pattern_id];
			let pattern = pattern_map.entry(beat.id).or_insert(random_pattern);

			println!("pattern:{}", pattern.description.to_owned());
			let mut last_note_time_in_beats = 0.0;

			let note_type = rand::thread_rng().gen_range(0, 2);
			for pattern_note in &pattern.notes {
				let mut beat_next_id = beat_id + pattern_note.beat_time_rel;
				if beat_next_id >= beats.len() {
					beat_next_id = beats.len() - 1;
				}
				let note_time_in_beats =
					(beats[beat_next_id].peak_time_sec / 60.0) * beats_per_minute;
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
					beat.id,
					pattern_end_time,
					highest_pitch, //highest pitch, not really using it but don't want it to spawn walls
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
				let wall_time_in_beats =
					(beats[beat_next_id].peak_time_sec / 60.0) * beats_per_minute;

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
			}}],\"_events\": [],
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

	writer.start_file("ExpertPlus.json", FileOptions::default())?;
	writer.write_all(song_json.as_bytes())?;

	println!("Adding cover");
	let cover = include_bytes!("./song/cover.jpg");
	writer.start_file("cover.jpg", FileOptions::default())?;
	writer.write_all(cover)?;
	println!("Adding info");
	let info = include_bytes!("./song/info.json");
	writer.start_file("info.json", FileOptions::default())?;
	writer.write_all(info)?;
	println!("Adding song");
	let song = include_bytes!("./song/song.ogg");
	writer.start_file("song.ogg", FileOptions::default())?;
	writer.write_all(song)?;
	println!("Adding done");
	writer.finish()?;
	Ok(())
}
