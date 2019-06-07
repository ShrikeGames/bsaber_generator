# Detect audio peaks with Librosa (https://librosa.github.io/librosa/)

# imports
from __future__ import print_function
import librosa
import numpy as np
import datetime

# Load local audio file
y, sr = librosa.load('src/song/song.ogg')

# Get file duration in seconds
duration = librosa.get_duration(y)

# Print duration to console
print("File duration(s): ", str(datetime.timedelta(seconds=duration)))

# Find peaks
onset_env = librosa.onset.onset_strength(y=y, sr=sr,
                                         hop_length=512,
                                         aggregate=np.median)
peaks = librosa.util.peak_pick(onset_env, 1, 5, 8, 8, 0.01, 1)

# Print peaks list to console
print('Peaks detected at: ', librosa.frames_to_time(peaks, sr=sr))


# Create CSV output
peak_times = librosa.frames_to_time(peaks, sr=sr)
#librosa.output.times_csv('peak_times.csv', peak_times)


#y1, sr1 = librosa.load('song.ogg')
#duration = librosa.get_duration(y1)

pitches, magnitudes = librosa.core.piptrack(y=y, sr=sr, fmin=10, fmax=1600, hop_length=512)

def detect_pitch(y, sr, t):
  index = magnitudes[:, int(t)].argmax()
  pitch = pitches[index, int(t)]

  return pitch

text_file = open("src/song/peak_times.txt", "w")
for peak_time in peak_times:
	text_file.write("%s %s\r\n" % (peak_time, detect_pitch(y, sr, peak_time)))
text_file.close()

# Complete message
print("Peak times output to peak_times.csv. \n Process complete.")

