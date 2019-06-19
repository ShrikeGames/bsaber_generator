# bsaber_generator
BeatSaber Custom Map Auto-Generator

* A project to auto-generate BeatSaber maps in the format that bsaber.com uses.

* Likely will be rough and need some improvements manually but would make starting maps far easier.

* Add your /src/song/song.ogg

* Modify /src/song/info.json with your song details

* Use the Python script to generate the peak timings and pitches:

* python peaks-detection.py

* Python requires:
librosa, numpy, pydub

* Run the Rust program with:

* cargo run

* Rust will download its requirements automatically.

Example Map (Play in beat saber! Be warned it's bad)

* https://bsaber.com/songs/538f/


TODOs

* Need to boost audio before making a map, apparently in-game it's way too quiet otherwise.

* Need to drastically improve the peak detection / other inputs so they map better to the actual song.

* Make a system to scrape together existing patterns from good maps to use as pre-set patterns.

* Find a better test song that will fit the style better, or has clearer beats.