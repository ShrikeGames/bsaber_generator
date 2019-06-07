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

fn main() -> std::io::Result<()> {
	println!("Start map creation");

	match create_bsaber_map() {
		Ok(_) => {
			println!("Map created");
		}
		Err(e) => {
			println!("Failed to create map. Error: {}", e);
		}
	}
	println!("Start zipping");
	//fs::remove_file("output/song.zip")?;
	let mut file = File::create("output/song.zip").expect("Couldn't create file");
	create_zip_archive(&mut file).expect("Couldn't create archive");
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

/*
*
*/
fn create_bsaber_map() -> std::io::Result<()> {
	fs::remove_file("src/song/ExpertPlus.json")?;
	let mut file = File::create("src/song/ExpertPlus.json")?;
	
	//read info.json
	let mut config_file = File::open("src/song/info.json").unwrap();
	let mut config_data = String::new();
	config_file.read_to_string(&mut config_data).unwrap();
	let config_json: JsonConfig = serde_json::from_str(&config_data[..])?;

	let version = "1.0.0";
	//get configs from info.json
	let beats_per_minute: f64 = config_json.beatsPerMinute;
	let beats_per_bar = config_json.beatsPerBar;
	let note_jump_speed = config_json.noteJumpSpeed;
	let shuffle = config_json.shuffle;
	let shuffle_period = config_json.shufflePeriod;
	let time: f64 = config_json.duration_seconds;

	//start of ExperPlus.json
	let mut contents: String = format!(
		"{{\"_version\": \"{}\",
			\"_beatsPerMinute\": {},
			\"_beatsPerBar\": {},
			\"_noteJumpSpeed\": {},
			\"_shuffle\": {},
			\"_shufflePeriod\": {},
			\"_time'\": {},
			\"_songTimeOffset\": 0.0,
			\"_notes\": [",
		version, beats_per_minute, beats_per_bar, note_jump_speed, shuffle, shuffle_period, time
	)
	.to_owned();

	//read the peak times and pitches
	let peak_times_file = File::open("src/song/peak_times.txt")?;
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
	//generate the map
	contents = generate_map(
		contents,
		&peak_times,
		&wall_times,
		&peak_pitches,
		highest_pitch,
		beats_per_minute,
		time,
	);
	//write it all out to file
	file.write_all(contents.as_bytes())?;
	drop(file);
	Ok(())
}
/*
* Generate a bsaber map using the information provided
*/
fn generate_map(
	mut contents: String,
	peak_times: &[f64],
	wall_times: &[f64],
	peak_pitches: &[f64],
	highest_pitch: f64,
	beats_per_minute: f64,
	time: f64,
) -> String {
	//track which beat were on using an id
	let mut id: usize = 0;
	//track where we last placed any block
	let mut last_x = 0;
	let mut last_y = 0;
	//track where we last placed a LEFT SABER block
	let mut last_xl = 0;
	let mut last_yl = 0;
	//trakc where we last place a RIGHT saber block
	let mut last_xr = 0;
	let mut last_yr = 0;
	//iterate over the peak times
	for peak_time in peak_times {
		//get the related pitch value
		let peak_pitch = peak_pitches[id];
		//now increase our id index
		id += 1;
		//if the pitch is high enough
		if peak_pitch >= MIN_PITCH {
			//value information:
			//index = left  0-4  right
			//layer = bottom  0-4  top
			//note type = 0 left, 1 right, 2 bomb
			//cut_direction = 0 up, 1 down, 2 left, 3 right, 4 NW, 5 NE, 6 SW, 7 SE, 8 omni-directional
			//get information for a new note
			let (time_beats, x, y, note_type, cut_direction, lrx, lry, lrxl, lryl, lrxr, lryr) =
				get_note_information(
					*peak_time,
					peak_pitch,
					highest_pitch,
					beats_per_minute,
					&last_x,
					&last_y,
					&last_xl,
					&last_yl,
					&last_xr,
					&last_yr,
				);
			//update the last placed block positions
			last_x = lrx;
			last_y = lry;
			last_xl = lrxl;
			last_yl = lryl;
			last_xr = lrxr;
			last_yr = lryr;
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
			);
			//add the note to the json string
			contents.push_str(&note);
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
	let mut start_time = 0.0;
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
			//track the start time with a buffer
			start_time = peak_time + WALL_SPACING;
			found_start = true;
		} else if found_start {
			//peak occured so we can figure out the distance of the wall
			//this is our wall end time, minus a buffer
			let end_time = peak_time - WALL_SPACING;
			//convert to beats (from seconds)
			let time_beats = (start_time / 60.0) * beats_per_minute;
			//println!("time_beats:{}", time_beats);
			//specs for our wall
			let mut line_index = 0;
			let wall_type = 0; //0 vertical 1 horizontal?
			let width = 0.5;
			//calculate duration in number of beats (not seconds)
			let duration = ((end_time - start_time) / 60.0) * beats_per_minute;
			//println!("duration (beats):{}", duration);
			//only make a wall if it's reasonably sized
			if duration >= WALL_SPACING * 2.0 {
				//make left side wall
				let wall: String = format!(
					"{{
							\"_time\": {},
							\"_lineIndex\": {},
							\"_type\": {},
							\"_duration\": {},
							\"_width\": {}
						}},",
					time_beats, line_index, wall_type, duration, width,
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
					time_beats, line_index, wall_type, duration, width,
				)
				.to_owned();
				//add to obstacles
				obstacles.push_str(&wall2);
				//now free us to find another wall start position
				found_start = false;
			}
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
			\"_cutDirection\": {}
		}},",
		id, peak_time, peak_pitch, time_beats, x, y, note_type, cut_direction,
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
	last_x: &i64,
	last_y: &i64,
	last_xl: &i64,
	last_yl: &i64,
	last_xr: &i64,
	last_yr: &i64,
) -> (f64, i64, i64, i64, i64, i64, i64, i64, i64, i64, i64) {
	//pick randomly between left and right saber
	let note_type = rand::thread_rng().gen_range(0, 2);
	let mut cut_direction = 8;
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
	//TODO make 4 a constant
	let mut y = ((peak_pitch / highest_pitch) * f64::from(4)) as i64;
	//if we randomly picked the same coordinates as our last block (either saber)
	//then we're going to adjust its position
	while x == *last_x && y == *last_y {
		//initially could be any cut direction (8 cardinal directions)
		//cut_direction = 0 N, 1 S, 2 W, 3 E, 4 NW, 5 NE, 6 SW, 7 SE, 8 omni
		//chosen at random
		let random_direction = rand::thread_rng().gen_range(0, 8);
		//based on the direction picked we'll change the direction of the noteblock
		//this makes it easier to chain notes together for the player
		if random_direction == 0 {
			//N
			y += 1;
			cut_direction = 0;
		} else if random_direction == 1 {
			//NE
			x += 1;
			y += 1;
			cut_direction = 5;
		} else if random_direction == 2 {
			//E
			x += 1;
			cut_direction = 3;
		} else if random_direction == 3 {
			//SE
			x += 1;
			y -= 1;
			cut_direction = 7;
		} else if random_direction == 4 {
			//S
			y -= 1;
			cut_direction = 1;
		} else if random_direction == 5 {
			//SW
			x -= 1;
			y -= 1;
			cut_direction = 6;
		} else if random_direction == 6 {
			//W
			x -= 1;
			cut_direction = 2;
		} else if random_direction == 7 {
			//NW
			x -= 1;
			y += 1;
			cut_direction = 4;
		}
		if x < 0 {
			x = 2;
			cut_direction = 8;
		} else if x > 3 {
			x = 1;
			cut_direction = 8;
		}
		if y < 0 {
			y = 2;
			cut_direction = 8;
		} else if y > 3 {
			y = 1;
			cut_direction = 8;
		}
	}
	//locally store the last note position
	let mut lx = *last_x;
	let mut ly = *last_x;
	//if we have a left/right saber grab those specific values instead
	if note_type == 0 {
		lx = *last_xl;
		ly = *last_yl;
	} else if note_type == 1 {
		lx = *last_xr;
		ly = *last_yr;
	}

	//compare to our last note position
	//change the direction of the block based on our relative position
	//if to the right point us in some right-ward cut direction
	if x > lx {
		if y > ly {
			cut_direction = 5; //NE
		} else if y < ly {
			cut_direction = 7; //SE
		} else {
			cut_direction = 3; //E
		}
	} else if x < lx {
		if y > ly {
			cut_direction = 4; //NW
		} else if y < ly {
			cut_direction = 6; //SW
		} else {
			cut_direction = 2; //W
		}
	} else if y > ly {
		cut_direction = 0; //N
	} else if y < ly {
		cut_direction = 1; //S
	}

	//NOTE _time is IN BEATS, NOT SECONDS
	//update our last used coordinates for notes
	let time_beats = (peak_time / 60.0) * beats_per_minute;
	let last_x_return = x;
	let last_y_return = y;
	let mut last_xl_return = last_xl;
	let mut last_yl_return = last_yl;
	let mut last_xr_return = last_xr;
	let mut last_yr_return = last_yr;

	if note_type == 0 {
		last_xl_return = &x;
		last_yl_return = &y;
	} else if note_type == 1 {
		last_xr_return = &x;
		last_yr_return = &y;
	}

	(
		time_beats,
		x,
		y,
		note_type,
		cut_direction,
		last_x_return,
		last_y_return,
		*last_xl_return,
		*last_yl_return,
		*last_xr_return,
		*last_yr_return,
	)
}

/*
* Zip up the important files from /src/song
* This is the zip beat saber, bsaber, or the map previewer can use
 */
fn create_zip_archive<T: Seek + Write>(buf: &mut T) -> ZipResult<()> {
	let mut writer = ZipWriter::new(buf);
	println!("Adding ExpertPlus");
	let contents = include_bytes!("./song/ExpertPlus.json");
	writer.start_file("ExpertPlus.json", FileOptions::default())?;
	writer.write_all(contents)?;
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
