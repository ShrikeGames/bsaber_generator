extern crate ogg;
extern crate rand;
extern crate zip;

use rand::Rng;
use serde::{Deserialize, Serialize};
use std::fs;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Read;
use std::io::{Seek, Write};
use zip::result::ZipResult;
use zip::write::{FileOptions, ZipWriter};
const WALL_SPACING: f64 = 0.5;
const MIN_PITCH: f64 = 10.0;
const NUMBER_OF_PATTERNS: i64 = 6;
const PATTERN_CHANCE: i64 = 3;
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
	let output_folder = "/home/matt/Games/beatsaver-viewer-master/song.zip";
	//let output_folder = "output/song.zip";
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

#[derive(Serialize, Deserialize)]
#[allow(non_snake_case)] //they're not snake case in the json
struct PatternNotes {
	duration: f64,
	notes: Vec<PatternNote>,
}

#[derive(Serialize, Deserialize)]
#[allow(non_snake_case)] //they're not snake case in the json
struct PatternNote {
	x: i64,
	y: i64,
	cut_direction: i64,
	note_type: i64,
	beat_time_rel: f64,
	description: String,
}
fn get_config_file() -> JsonConfig {
	let mut config_file = File::open("src/song/info.json").unwrap();
	let mut config_data = String::new();
	config_file.read_to_string(&mut config_data).unwrap();
	let config_json: JsonConfig =
		serde_json::from_str(&config_data[..]).expect("Could not read config file");
	(config_json)
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

	let mut patterns = Vec::new();
	//load pattern json?
	for pattern_id in 0..NUMBER_OF_PATTERNS {
		let mut pattern_file =
			File::open(format!("src/patterns/pattern_{}.json", pattern_id)).unwrap();
		let mut pattern_json = String::new();
		pattern_file.read_to_string(&mut pattern_json).unwrap();

		let pattern_notes: PatternNotes =
			serde_json::from_str(&pattern_json[..]).expect("Could not read pattern note json");

		patterns.push(pattern_notes);
	}

	let version = "1.0.0";
	//get configs from info.json
	let beats_per_minute: f64 = config_json.beatsPerMinute;
	let beats_per_bar = config_json.beatsPerBar;
	let note_jump_speed = config_json.noteJumpSpeed;
	let shuffle = config_json.shuffle;
	let shuffle_period = config_json.shufflePeriod;
	let time: f64 = (config_json.duration_seconds / 60.0) * beats_per_minute;;

	//start of ExperPlus.json
	let mut contents: String = format!(
		"{{\"_version\": \"{}\",
			\"_beatsPerMinute\": {},
			\"_beatsPerBar\": {},
			\"_noteJumpSpeed\": {},
			\"_shuffle\": {},
			\"_shufflePeriod\": {},
			\"_time\": {},
			\"_songTimeOffset\": 0.0,
			\"_notes\": [",
		version, beats_per_minute, beats_per_bar, note_jump_speed, shuffle, shuffle_period, time
	)
	.to_owned();

	//read the peak times and pitches
	let peak_times_file =
		File::open("src/song/peak_times.txt").expect("Could not read peak times file");
	let peak_times_buffer = BufReader::new(&peak_times_file);
	//we'll keep track of times for the peaks and where walls might go
	let mut peak_times = Vec::new();
	let mut wall_times = Vec::new();
	//keep the extracted pitch values too
	let mut peak_pitches = Vec::new();
	//find the highest pitch while we're at it
	let mut highest_pitch: f64 = 0.0;
	//iterate over the file line by line
	for line in peak_times_buffer.lines() {
		let line = line.unwrap();
		let mut iter = line.split_whitespace();
		//1.0216780045351475 175.67802
		//first number is the time (in seconds), second number is the pitch measurement
		let peak_time = iter.next().unwrap();
		let peak_pitch = iter.next().unwrap();
		//convert to floats
		let peak_time_as_float = peak_time.parse::<f64>().unwrap();
		let peak_pitch_as_float = peak_pitch.parse::<f64>().unwrap();
		//store in vectors
		peak_times.push(peak_time_as_float);
		wall_times.push(peak_time_as_float);
		peak_pitches.push(peak_pitch_as_float);
		//track highest pitch
		if peak_pitch_as_float > highest_pitch {
			highest_pitch = peak_pitch_as_float;
		}
	}
	//ensure the wall times always have the song time (length/duration)
	wall_times.push(time);

	let mut processed_notes = Vec::new();
	//generate the map
	contents = generate_map(
		contents,
		&peak_times,
		&wall_times,
		&peak_pitches,
		&patterns,
		highest_pitch,
		beats_per_minute,
		time,
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
#[allow(clippy::too_many_arguments)]
fn generate_map(
	mut contents: String,
	peak_times: &[f64],
	wall_times: &[f64],
	peak_pitches: &[f64],
	patterns: &[PatternNotes],
	highest_pitch: f64,
	beats_per_minute: f64,
	time: f64,
	processed_notes: &mut Vec<PatternNote>,
) -> String {
	//track which beat were on using an id
	let mut id: usize = 0;
	let mut pattern_end_time: f64 = 0.0;

	//iterate over the peak times
	for peak_time in peak_times {
		//get the related pitch value
		let peak_pitch = peak_pitches[id];
		//now increase our id index
		id += 1;
		//if the pitch is high enough
		if peak_pitch >= MIN_PITCH && *peak_time > pattern_end_time {
			let use_pattern = rand::thread_rng().gen_range(0, 100);
			if use_pattern <= PATTERN_CHANCE {
				let pattern_id = rand::thread_rng().gen_range(0, patterns.len());
				let pattern_notes = &patterns[pattern_id];
				let note_type = rand::thread_rng().gen_range(0, 2);
				pattern_end_time = *peak_time + pattern_notes.duration;
				for pattern_note in &pattern_notes.notes {
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

					if pattern_y < 0 || pattern_x < 0 && !processed_notes.is_empty() {
						let prev_note_index = processed_notes.len() - 1;
						let prev_note = &processed_notes[prev_note_index];

						//if we randomly picked the same coordinates as our last block (either saber)
						//then we're going to adjust its position
						while pattern_x == prev_note.x && pattern_y == prev_note.y {
							pattern_y += rand::thread_rng().gen_range(-1, 2);
							pattern_x += rand::thread_rng().gen_range(-1, 2);
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
					let mut pattern_cut_direction = pattern_note.cut_direction;
					if pattern_cut_direction < 0 {
						pattern_cut_direction = rand::thread_rng().gen_range(0, 8);
					}

					let pattern_time_beat = pattern_note.beat_time_rel;
					let mut time_beats = (peak_time / 60.0) * beats_per_minute;
					if pattern_time_beat > 0.0 {
						time_beats += pattern_time_beat;
					}
					//create the note using the values we calculated
					let note: String = create_note_json(
						id,
						*peak_time,
						highest_pitch, //highest pitch, not really using it but don't want it to spawn walls
						time_beats,
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
						beat_time_rel: pattern_note.beat_time_rel,
						description: "Test".to_owned(),
					};
					processed_notes.push(processed_note);
				}
			} else {
				//value information:
				//index = left  0-4  right
				//layer = bottom  0-4  top
				//note type = 0 left, 1 right, 2 bomb
				//cut_direction = 0 up, 1 down, 2 left, 3 right, 4 NW, 5 NE, 6 SE, 7 SW, 8 omni-directional
				//get information for a new note
				let (time_beats, x, y, note_type, cut_direction) = get_note_information(
					*peak_time,
					peak_pitch,
					highest_pitch,
					beats_per_minute,
					processed_notes,
				);
				//update the last placed block positions
				//create the note using the values we calculated
				let note: String = create_note_json(
					id,
					*peak_time,
					peak_pitch,
					time_beats,
					x,
					y,
					note_type,
					cut_direction,
					"auto".to_owned(),
				);

				//add the note to the json string
				contents.push_str(&note);
				let processed_note = PatternNote {
					x,
					y,
					cut_direction,
					note_type,
					beat_time_rel: 0.0,
					description: "auto".to_owned(),
				};
				processed_notes.push(processed_note);
			}
		}
	}
	//generate walls
	contents = generate_walls(contents, wall_times, &peak_pitches, beats_per_minute, time);

	//return the completed json string
	(contents)
}
/*
* Generates walls to fill empty/low pitch areas
*/
fn generate_walls(
	mut contents: String,
	wall_times: &[f64],
	peak_pitches: &[f64],
	beats_per_minute: f64,
	time: f64,
) -> String {
	//obstacles json string
	let mut obstacles: String = "".to_string();
	//id index
	let mut id = 0;
	//track the start time of a wall, we'll want it to start at the first quiet part
	let mut start_time: f64 = 0.0;
	//flag if we've found a wall start or not
	let mut found_start = false;
	//iterate over the wall times (including the last time added manually)
	for peak_time in wall_times {
		//wall times has an extra wall at the end added on so the size i 1 larger
		let pitch = if id < peak_pitches.len() {
			peak_pitches[id]
		} else {
			0.0
		};
		//increment the id index
		id += 1;
		//if it's quiet and we need a wall start
		if pitch < MIN_PITCH && !found_start {
			//silence is starting
			//track the start time
			start_time = *peak_time;
			found_start = true;
			println!("start pitch:{}", pitch);
		} else if found_start && pitch >= MIN_PITCH {
			//peak occured so we can figure out the distance of the wall
			//this is our wall end time
			let end_time = peak_time;
			//convert to beats (from seconds)
			let time_beats = (start_time / 60.0) * beats_per_minute;
			let end_time_beats = (end_time / 60.0) * beats_per_minute;
			//specs for our wall
			let mut line_index = 0;
			let wall_type = 0; //0 vertical 1 horizontal?
			let width = 0.5;
			//calculate duration in number of beats (not seconds)
			let duration = end_time_beats - time_beats - (WALL_SPACING * 2.0);

			//only make a wall if it's reasonably sized
			if duration >= WALL_SPACING * 3.0 {
				println!("wall added");
				println!("end pitch:{}", pitch);
				println!("time_beats:{}", time_beats);
				println!("end_time_beats:{}", end_time_beats);
				println!("duration (beats):{}", duration);
				//make left side wall
				let wall: String = format!(
					"{{
						\"_time\": {},
						\"_lineIndex\": {},
						\"_type\": {},
						\"_duration\": {},
						\"_width\": {}
					}},",
					time_beats + WALL_SPACING,
					line_index,
					wall_type,
					duration,
					width,
				)
				.to_owned();
				//add to obstacles
				obstacles.push_str(&wall);
				//make right side wall
				line_index = 3;
				let wall2: String = format!(
					"{{
						\"_time\": {},
						\"_lineIndex\": {},
						\"_type\": {},
						\"_duration\": {},
						\"_width\": {}
					}},",
					time_beats + WALL_SPACING,
					line_index,
					wall_type,
					duration,
					width,
				)
				.to_owned();
				//add to obstacles
				obstacles.push_str(&wall2);
			}
			//now free us to find another wall start position
			//may not have found a space large enough for a wall but we still start over
			found_start = false;
			start_time = 0.0;
		}
	}

	//the end of the json file contents
	//note adding an extra obstacle with no size so we don't have a hanging comma
	let contents_end: String = format!(
		"{{
			\"_time\": {},
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
		time, obstacles
	)
	.to_owned();
	//add to json string
	contents.push_str(&contents_end);
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
* Get the information to place a new block based on logic.
* Will include the bulk of the logic for making maps.
*/
#[allow(clippy::too_many_arguments, clippy::cyclomatic_complexity)]
fn get_note_information(
	peak_time: f64,
	peak_pitch: f64,
	highest_pitch: f64,
	beats_per_minute: f64,
	processed_notes: &[PatternNote],
) -> (f64, i64, i64, i64, i64) {
	//pick randomly between left and right saber
	let note_type = rand::thread_rng().gen_range(0, 2);

	let mut x: i64 = 0;

	if note_type == 0 {
		//left
		//start the note on the two left columns (randomly)
		x = rand::thread_rng().gen_range(0, 2);
	} else if note_type == 1 {
		//right
		//start the note on the two right columns (randomly)
		x = rand::thread_rng().gen_range(2, 4);
	}
	//the y position/row is determined by comparing the pitch against the highest pitch note
	//4 vertical rows
	//higher pitch have chance to be higher
	let mut y =
		rand::thread_rng().gen_range(0, 1 + ((peak_pitch / highest_pitch) * f64::from(4)) as i64);

	//default to omni direction
	let mut cut_direction = CUT_OMNI;
	if !processed_notes.is_empty() {
		let mut prev_note_index = processed_notes.len() - 1;
		let mut prev_note = &processed_notes[prev_note_index];

		//if we randomly picked the same coordinates as our last block (either saber)
		//then we're going to adjust its position
		while x == prev_note.x && y == prev_note.y {
			y += rand::thread_rng().gen_range(-1, 2);
			if note_type == 0 {
				x -= rand::thread_rng().gen_range(1, 3);
			} else if note_type == 1 {
				x += rand::thread_rng().gen_range(1, 3);
			} else {
				x += rand::thread_rng().gen_range(-1, 2);
			}
		}
		//ensure they are in valid positions
		if x > 3 {
			x = 3;
		} else if x < 0 {
			x = 0;
		}
		if y > 3 {
			y = 3;
		} else if y < 0 {
			y = 0;
		}
		//now we'll look specifically for the last note of our particular saber to decide on our cut direction
		while note_type != prev_note.note_type && prev_note_index > 0 {
			prev_note_index -= 1;
			prev_note = &processed_notes[prev_note_index];
		}
		//compare to our last note position
		//change the direction of the block based on our relative position
		//if to the right point us in some right-ward cut direction
		//cut_direction =
		//0 up
		//1 down
		//2 left
		//3 right
		//4 NW
		//5 NE
		//6 SW
		//7 SE
		//8 omni-directional
		if x == prev_note.x {
			if y > prev_note.y + 1 {
				cut_direction = CUT_UP;
			} else if y < prev_note.y - 1 {
				cut_direction = CUT_DOWN;
			} else {
				cut_direction = CUT_OMNI;
			}
		} else if x > prev_note.x + 1 {
			if y > prev_note.y + 1 {
				cut_direction = CUT_NE;
			} else if y < prev_note.y - 1 {
				cut_direction = CUT_SE;
			} else {
				cut_direction = CUT_OMNI;
			}
		} else if x < prev_note.x - 1 {
			if y > prev_note.y + 1 {
				cut_direction = CUT_NW;
			} else if y < prev_note.y - 1 {
				cut_direction = CUT_SW;
			} else {
				cut_direction = CUT_OMNI;
			}
		} else if x > prev_note.x {
			if y > prev_note.y + 1 || y < prev_note.y - 1 {
				cut_direction = CUT_OMNI;
			} else {
				cut_direction = CUT_RIGHT;
			}
		} else if x < prev_note.x {
			if y > prev_note.y + 1 || y < prev_note.y - 1 {
				cut_direction = CUT_OMNI;
			} else {
				cut_direction = CUT_LEFT;
			}
		}
		//special case for the bottom row so we aren't going side to side too much
		//would rather alternate up and down
		if y == 0 && prev_note.y == 0 && (cut_direction == CUT_LEFT || cut_direction == CUT_RIGHT) {
			if prev_note.cut_direction == CUT_UP || prev_note.cut_direction == CUT_OMNI {
				cut_direction = CUT_DOWN;
			} else if prev_note.cut_direction == CUT_DOWN {
				cut_direction = CUT_UP;
			} else {
				cut_direction = CUT_OMNI;
			}
		}
	}

	//NOTE _time is IN BEATS, NOT SECONDS
	//update our last used coordinates for notes
	let time_beats = (peak_time / 60.0) * beats_per_minute;

	(time_beats, x, y, note_type, cut_direction)
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
