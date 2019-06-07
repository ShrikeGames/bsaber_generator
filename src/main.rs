extern crate ogg;
extern crate rand;
extern crate zip;

use rand::Rng;
use std::fs;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::io::{Seek, Write};
use zip::result::ZipResult;
use zip::write::{FileOptions, ZipWriter};
const WALL_SPACING: f64 = 0.5;
const MIN_PITCH: f64 = 10.0;
#[allow(clippy::too_many_arguments)]
fn create_note(
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
	let note_type = rand::thread_rng().gen_range(0, 2);
	let mut cut_direction = 8;
	let mut x: i64 = 0;
	if note_type == 0 {
		//left
		x = rand::thread_rng().gen_range(0, 2);
	} else if note_type == 1 {
		x = rand::thread_rng().gen_range(2, 4);
	}
	let mut y = ((peak_pitch / highest_pitch) * f64::from(4)) as i64;
	while x == *last_x && y == *last_y {
		let random_direction = rand::thread_rng().gen_range(0, 8);
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
	//cut_direction = 0 up, 1 down, 2 left, 3 right, 4 NW, 5 NE, 6 SW, 7 SE, 8 omni
	let mut lx = *last_x;
	let mut ly = *last_x;

	if note_type == 0 {
		lx = *last_xl;
		ly = *last_yl;
	} else if note_type == 1 {
		lx = *last_xr;
		ly = *last_yr;
	}

	if x > lx {
		if y > ly {
			cut_direction = 5;
		} else if y < ly {
			cut_direction = 7;
		} else {
			cut_direction = 3;
		}
	} else if x < lx {
		if y > ly {
			cut_direction = 4;
		} else if y < ly {
			cut_direction = 6;
		} else {
			cut_direction = 2;
		}
	} else if y > ly {
		cut_direction = 0;
	} else if y < ly {
		cut_direction = 1;
	}

	//NOTE _time is IN BEATS, NOT SECONDS
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
fn generate_map(
	mut contents: String,
	peak_times: &[f64],
	wall_times: &[f64],
	peak_pitches: &[f64],
	highest_pitch: f64,
	beats_per_minute: f64,
	time: f64,
) -> String {
	let mut id: usize = 0;
	let mut last_x = 0;
	let mut last_y = 0;
	let mut last_xl = 0;
	let mut last_yl = 0;
	let mut last_xr = 0;
	let mut last_yr = 0;

	for peak_time in peak_times {
		let peak_pitch = peak_pitches[id];

		id += 1;
		if peak_pitch >= MIN_PITCH {
			//index = left  0-4  right
			//layer = bottom  0-4  top
			//note type = 0 left, 1 right, 2 bomb
			//cut_direction = 0 up, 1 down, 2 left, 3 right, 4 NW, 5 NE, 6 SW, 7 SE, 8 omni

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
			last_x = lrx;
			last_y = lry;
			last_xl = lrxl;
			last_yl = lryl;
			last_xr = lrxr;
			last_yr = lryr; //12sec at 130bpm =? beats
				//0.2minutes*130=26
			let note: String = create_note(
				id,
				*peak_time,
				peak_pitch,
				time_beats,
				x,
				y,
				note_type,
				cut_direction,
			);

			contents.push_str(&note);
		}
	}
	contents = generate_walls(contents, wall_times, &peak_pitches, beats_per_minute, time);
	(contents)
}
fn generate_walls(
	mut contents: String,
	wall_times: &[f64],
	peak_pitches: &[f64],
	beats_per_minute: f64,
	time: f64,
) -> String {
	let mut obstacles: String = "".to_string();
	let mut id = 0;
	let mut start_time = 0.0;
	let mut found_start = false;
	for peak_time in wall_times {
		//wall times has an extra wall at the end added on so the size i 1 larger
		let pitch = if id < peak_pitches.len() {
			peak_pitches[id]
		} else {
			0.0
		};
		id += 1;

		if pitch < MIN_PITCH && !found_start{
			//silence is starting
			start_time = peak_time + WALL_SPACING;
			found_start = true;
		} else if found_start {
			//peak occured so we can figure out the distance of the wall
			let end_time = peak_time - WALL_SPACING;
			let time_beats = (start_time / 60.0) * beats_per_minute;
			//println!("time_beats:{}", time_beats);
			//let random_side = rand::thread_rng().gen_range(0, 2);
			let mut line_index = 0;
			let wall_type = 0;
			let width = 0.5;
			let duration = ((end_time - start_time) / 60.0) * beats_per_minute;
			//println!("duration (beats):{}", duration);
			//only make a wall if it's reasonably sized
			if duration >= WALL_SPACING*2.0 {
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
				obstacles.push_str(&wall);
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
				obstacles.push_str(&wall2);
				found_start = false;
				
			}
		}
	}
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
	contents.push_str(&contents_end);
	(contents)
}
fn create_song() -> std::io::Result<()> {
	fs::remove_file("src/song/ExpertPlus.json")?;
	let mut file = File::create("src/song/ExpertPlus.json")?;
	let version = "1.0.0";
	let beats_per_minute: f64 = 150.0;
	let beats_per_bar = 16;
	let note_jump_speed = 15;
	let shuffle = 1;
	let shuffle_period = 0.1;
	let time: f64 = 227.0;

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

	let peak_times_file = File::open("src/song/peak_times.txt")?;
	let peak_times_buffer = BufReader::new(&peak_times_file);

	let mut peak_times = Vec::new();
	let mut wall_times = Vec::new();
	let mut peak_pitches = Vec::new();
	let mut highest_pitch: f64 = 0.0;
	for line in peak_times_buffer.lines() {
		let line = line.unwrap();
		let mut iter = line.split_whitespace();
		let peak_time = iter.next().unwrap();
		let peak_pitch = iter.next().unwrap();

		let peak_time_as_float = peak_time.parse::<f64>().unwrap();
		let peak_pitch_as_float = peak_pitch.parse::<f64>().unwrap();

		peak_times.push(peak_time_as_float);
		wall_times.push(peak_time_as_float);
		peak_pitches.push(peak_pitch_as_float);
		if peak_pitch_as_float > highest_pitch {
			highest_pitch = peak_pitch_as_float;
		}
	}
	wall_times.push(time);

	contents = generate_map(
		contents,
		&peak_times,
		&wall_times,
		&peak_pitches,
		highest_pitch,
		beats_per_minute,
		time,
	);

	file.write_all(contents.as_bytes())?;
	drop(file);
	Ok(())
}
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
fn main() -> std::io::Result<()> {
	//	let mut file = File::open("src/song/ExpertPlus.json")?;
	//	let mut contents = String::new();
	//	file.read_to_string(&mut contents)?;
	//	println!("{}", contents);

	println!("Start map creation");

	match create_song() {
		Ok(_) => {
			println!("Map created");
			println!("Start zipping");
			fs::remove_file("/home/matt/Games/beatsaver-viewer-master/song.zip")?;
			let mut file = File::create("/home/matt/Games/beatsaver-viewer-master/song.zip")
				.expect("Couldn't create file");
			create_zip_archive(&mut file).expect("Couldn't create archive");
			println!("Finished zipping");
		}
		Err(e) => {
			println!("Failed to create map. Error: {}", e);
		}
	}

	Ok(())
}
