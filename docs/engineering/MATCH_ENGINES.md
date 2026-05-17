# Match engines

A match engine is a module that takes indexed files and produces groups of related files.

## Required output for every engine

Each group should include:

- group ID
- items
- match type
- match reason
- confidence score if applicable
- false-positive risk level
- suggested keep item
- engine-specific metadata

## Engine 1: Exact duplicates

Status: partially implemented.

Rules:

- file sizes must match
- full content hashes must match
- optional byte verification may be enabled

False-positive risk:

- very low with cryptographic hash
- very low with byte verification
- low with fast non-cryptographic hash, but byte verification is recommended before destructive action

## Engine 2: Similar filenames

Status: planned.

Possible methods:

- exact filename
- normalized filename
- filename without extension
- token overlap
- Levenshtein distance
- Ratcliff-Obershelp style similarity
- ignored punctuation
- ignored bracketed tags
- ignored year/date patterns

False-positive risk:

- medium to high depending on threshold

## Engine 3: Similar images

Status: planned.

Possible methods:

- exact binary hash
- decoded pixel hash
- perceptual hash
- color histogram
- EXIF timestamp
- RAW + JPEG pairing
- rotation/flip-aware comparison

False-positive risk:

- low for exact decoded-pixel match
- medium for perceptual hash
- high for EXIF-only matching

## Engine 4: Similar videos

Status: planned.

Possible methods:

- exact binary hash
- same or near-same duration
- sampled frame perceptual hashes
- optional audio fingerprint
- resolution/codec ignored

False-positive risk:

- medium; depends heavily on sampling strategy and thresholds

## Engine 5: Similar music/audio

Status: planned.

Possible methods:

- exact binary hash
- metadata tag match
- duration match
- audio fingerprint
- waveform/sample checksum

False-positive risk:

- low for audio fingerprint with sane thresholds
- medium to high for tags only

## Engine 6: Duplicate folders

Status: planned.

Possible methods:

- same tree structure
- same filenames
- same sizes
- same content hashes
- same tree after ignored files

False-positive risk:

- low when content hashes are used
- medium when names/sizes only are used
