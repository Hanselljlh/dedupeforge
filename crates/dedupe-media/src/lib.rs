use anyhow::{Context, Result};
use exif::{In, Reader, Tag, Value};
use image::imageops::{self, FilterType};
use image::{DynamicImage, GrayImage};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImageAnalysis {
    pub perceptual_hash_hex: String,
    pub perceptual_hashes_hex: Vec<String>,
    pub exif_date: Option<String>,
    pub transform_count: usize,
}

#[derive(Clone, Debug)]
pub struct ImageComparison {
    pub distance: u32,
    pub matched_transform: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MediaToolConfig {
    pub ffmpeg_bin: String,
    pub ffprobe_bin: String,
}

impl Default for MediaToolConfig {
    fn default() -> Self {
        Self {
            ffmpeg_bin: std::env::var("DEDUPEFORGE_FFMPEG")
                .unwrap_or_else(|_| "ffmpeg".to_string()),
            ffprobe_bin: std::env::var("DEDUPEFORGE_FFPROBE")
                .unwrap_or_else(|_| "ffprobe".to_string()),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VideoAnalysis {
    pub fingerprint_hex: String,
    pub duration_seconds: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AudioAnalysis {
    pub fingerprint_hex: String,
    pub duration_seconds: f64,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProbeMetadata {
    pub duration_seconds: f64,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
}

pub fn supported_image_extension(path: &Path) -> bool {
    matches!(
        normalized_extension(path).as_deref(),
        Some("jpg" | "jpeg" | "png" | "gif" | "bmp" | "webp" | "tif" | "tiff")
    )
}

pub fn supported_raw_extension(path: &Path) -> bool {
    matches!(
        normalized_extension(path).as_deref(),
        Some("cr2" | "cr3" | "nef" | "arw" | "dng" | "rw2" | "orf" | "raf" | "pef" | "srw")
    )
}

pub fn analyze_image(
    path: &Path,
    hash_size: u32,
    rotation_invariant: bool,
) -> Result<ImageAnalysis> {
    let image =
        image::open(path).with_context(|| format!("failed to decode image {}", path.display()))?;
    let hashes = perceptual_hash_variants(&image, hash_size, rotation_invariant);
    let perceptual_hashes_hex = hashes.iter().map(hex::encode).collect::<Vec<_>>();
    let perceptual_hash_hex = perceptual_hashes_hex.first().cloned().unwrap_or_default();

    Ok(ImageAnalysis {
        perceptual_hash_hex,
        perceptual_hashes_hex,
        exif_date: read_exif_date(path).ok().flatten(),
        transform_count: hashes.len(),
    })
}

pub fn compare_images(
    left: &Path,
    right: &Path,
    hash_size: u32,
    rotation_invariant: bool,
) -> Result<ImageComparison> {
    let left_image =
        image::open(left).with_context(|| format!("failed to decode image {}", left.display()))?;
    let right_image = image::open(right)
        .with_context(|| format!("failed to decode image {}", right.display()))?;

    let left_hashes = perceptual_hash_variants(&left_image, hash_size, rotation_invariant);
    let right_hashes = perceptual_hash_variants(&right_image, hash_size, rotation_invariant);

    let mut best = ImageComparison {
        distance: u32::MAX,
        matched_transform: 0,
    };

    for (left_index, left_hash) in left_hashes.iter().enumerate() {
        for right_hash in &right_hashes {
            let distance = hamming_distance(left_hash, right_hash);
            if distance < best.distance {
                best = ImageComparison {
                    distance,
                    matched_transform: left_index,
                };
            }
        }
    }

    Ok(best)
}

pub fn compare_hashes_hex(left: &str, right: &str) -> Result<u32> {
    let left_bytes = hex::decode(left)?;
    let right_bytes = hex::decode(right)?;
    Ok(hamming_distance(&left_bytes, &right_bytes))
}

pub fn supported_video_extension(path: &Path) -> bool {
    matches!(
        normalized_extension(path).as_deref(),
        Some("mp4" | "mkv" | "avi" | "mov" | "wmv" | "m4v" | "webm" | "mpg" | "mpeg")
    )
}

pub fn supported_audio_extension(path: &Path) -> bool {
    matches!(
        normalized_extension(path).as_deref(),
        Some("mp3" | "flac" | "wav" | "m4a" | "aac" | "ogg" | "opus" | "wma")
    )
}

pub fn media_tools_available(config: &MediaToolConfig) -> Result<()> {
    tool_available(&config.ffprobe_bin)
        .with_context(|| format!("required dependency not available: {}", config.ffprobe_bin))?;
    tool_available(&config.ffmpeg_bin)
        .with_context(|| format!("required dependency not available: {}", config.ffmpeg_bin))?;
    Ok(())
}

pub fn analyze_video(path: &Path, config: &MediaToolConfig) -> Result<VideoAnalysis> {
    media_tools_available(config)?;
    let metadata = probe_metadata(path, config)?;
    let fingerprint_hex = fingerprint_media(
        path,
        config,
        &[
            "-v",
            "error",
            "-i",
            path.to_string_lossy().as_ref(),
            "-map",
            "0:v:0",
            "-vf",
            "fps=1,scale=32:32:flags=bilinear,format=gray",
            "-frames:v",
            "6",
            "-f",
            "rawvideo",
            "-",
        ],
    )?;

    Ok(VideoAnalysis {
        fingerprint_hex,
        duration_seconds: metadata.duration_seconds,
    })
}

pub fn analyze_audio(path: &Path, config: &MediaToolConfig) -> Result<AudioAnalysis> {
    media_tools_available(config)?;
    let metadata = probe_metadata(path, config)?;
    let fingerprint_hex = fingerprint_media(
        path,
        config,
        &[
            "-v",
            "error",
            "-i",
            path.to_string_lossy().as_ref(),
            "-map",
            "0:a:0",
            "-ac",
            "1",
            "-ar",
            "8000",
            "-t",
            "30",
            "-f",
            "s16le",
            "-",
        ],
    )?;

    Ok(AudioAnalysis {
        fingerprint_hex,
        duration_seconds: metadata.duration_seconds,
        title: metadata.title,
        artist: metadata.artist,
        album: metadata.album,
    })
}

pub fn probe_metadata(path: &Path, config: &MediaToolConfig) -> Result<ProbeMetadata> {
    media_tools_available(config)?;
    let mut command = Command::new(&config.ffprobe_bin);
    command
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration:format_tags=title,artist,album",
            "-of",
            "default=noprint_wrappers=1:nokey=0",
        ])
        .arg(path);
    let output = command_output_with_timeout(command, Duration::from_secs(20))
        .with_context(|| format!("failed to execute {}", config.ffprobe_bin))?;
    if !output.status.success() {
        anyhow::bail!(
            "{} failed for {}: {}",
            config.ffprobe_bin,
            path.display(),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(parse_ffprobe_key_values(&String::from_utf8_lossy(
        &output.stdout,
    )))
}

fn fingerprint_media(path: &Path, config: &MediaToolConfig, args: &[&str]) -> Result<String> {
    let mut command = Command::new(&config.ffmpeg_bin);
    command.args(args);
    let output = command_output_with_timeout(command, Duration::from_secs(30))
        .with_context(|| format!("failed to execute {}", config.ffmpeg_bin))?;
    if !output.status.success() {
        anyhow::bail!(
            "{} failed for {}: {}",
            config.ffmpeg_bin,
            path.display(),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(blake3::hash(&output.stdout).to_hex().to_string())
}

fn command_output_with_timeout(mut command: Command, timeout: Duration) -> Result<Output> {
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let start = Instant::now();

    loop {
        if child.try_wait()?.is_some() {
            return Ok(child.wait_with_output()?);
        }
        if start.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            anyhow::bail!("command timed out after {} seconds", timeout.as_secs());
        }
        thread::sleep(Duration::from_millis(50));
    }
}

fn tool_available(tool: &str) -> Result<()> {
    let mut command = Command::new(tool);
    command.arg("-version");
    let output = command_output_with_timeout(command, Duration::from_secs(10))
        .with_context(|| format!("failed to launch {}", tool))?;
    if output.status.success() {
        Ok(())
    } else {
        anyhow::bail!("{} returned a non-zero status", tool);
    }
}

fn parse_ffprobe_key_values(text: &str) -> ProbeMetadata {
    let mut duration_seconds = 0.0f64;
    let mut title = None;
    let mut artist = None;
    let mut album = None;

    for line in text.lines() {
        if let Some((key, value)) = line.split_once('=') {
            match key.trim() {
                "duration" => {
                    duration_seconds = value.trim().parse::<f64>().unwrap_or_default();
                }
                "TAG:title" => title = non_empty(value),
                "TAG:artist" => artist = non_empty(value),
                "TAG:album" => album = non_empty(value),
                _ => {}
            }
        }
    }

    ProbeMetadata {
        duration_seconds,
        title,
        artist,
        album,
    }
}

fn non_empty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

pub fn read_exif_date(path: &Path) -> Result<Option<String>> {
    let file = File::open(path).with_context(|| format!("failed to open {}", path.display()))?;
    let mut reader = BufReader::new(file);
    let exif = Reader::new().read_from_container(&mut reader).ok();
    let Some(exif) = exif else {
        return Ok(None);
    };

    for tag in [Tag::DateTimeOriginal, Tag::DateTimeDigitized, Tag::DateTime] {
        if let Some(field) = exif.get_field(tag, In::PRIMARY) {
            if let Value::Ascii(values) = &field.value {
                if let Some(first) = values.first() {
                    let value = String::from_utf8_lossy(first).trim().to_string();
                    if !value.is_empty() {
                        return Ok(Some(value));
                    }
                }
            }
        }
    }

    Ok(None)
}

pub fn perceptual_hash_variants(
    image: &DynamicImage,
    hash_size: u32,
    rotation_invariant: bool,
) -> Vec<Vec<u8>> {
    let base = resize_grayscale(image, hash_size);
    let mut variants = vec![average_hash(&base)];

    if rotation_invariant {
        let rotated_90 = imageops::rotate90(&base);
        let rotated_180 = imageops::rotate180(&base);
        let rotated_270 = imageops::rotate270(&base);
        let flipped = imageops::flip_horizontal(&base);

        variants.push(average_hash(&rotated_90));
        variants.push(average_hash(&rotated_180));
        variants.push(average_hash(&rotated_270));
        variants.push(average_hash(&flipped));
    }

    variants
}

fn resize_grayscale(image: &DynamicImage, hash_size: u32) -> GrayImage {
    image
        .grayscale()
        .resize_exact(hash_size, hash_size, FilterType::Triangle)
        .to_luma8()
}

fn average_hash(image: &GrayImage) -> Vec<u8> {
    let pixels = image
        .pixels()
        .map(|pixel| pixel[0] as u64)
        .collect::<Vec<_>>();
    let average = pixels.iter().sum::<u64>() / pixels.len().max(1) as u64;

    let mut bits = Vec::with_capacity(pixels.len().div_ceil(8));
    let mut current = 0u8;
    let mut count = 0u8;

    for pixel in pixels {
        current <<= 1;
        if pixel >= average {
            current |= 1;
        }
        count += 1;
        if count == 8 {
            bits.push(current);
            current = 0;
            count = 0;
        }
    }

    if count != 0 {
        current <<= 8 - count;
        bits.push(current);
    }

    bits
}

fn hamming_distance(left: &[u8], right: &[u8]) -> u32 {
    left.iter()
        .zip(right.iter())
        .map(|(a, b)| (a ^ b).count_ones())
        .sum()
}

fn normalized_extension(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgb};
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("dedupeforge-media-{unique}-{name}"));
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn sample_image(path: &Path, seed: u8) {
        let mut image = ImageBuffer::<Rgb<u8>, Vec<u8>>::new(32, 32);
        for (x, y, pixel) in image.enumerate_pixels_mut() {
            let value = seed
                .wrapping_add((x as u8).wrapping_mul(3))
                .wrapping_add((y as u8).wrapping_mul(5));
            *pixel = Rgb([value, value / 2, 255u8.wrapping_sub(value)]);
        }
        image.save(path).unwrap();
    }

    #[test]
    fn average_hash_matches_identical_images() {
        let root = temp_dir("hash");
        let a = root.join("a.png");
        let b = root.join("b.png");
        sample_image(&a, 10);
        fs::copy(&a, &b).unwrap();

        let comparison = compare_images(&a, &b, 8, false).unwrap();
        assert_eq!(comparison.distance, 0);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rotation_invariant_mode_reduces_distance() {
        let root = temp_dir("rotate");
        let a = root.join("a.png");
        let b = root.join("b.png");
        sample_image(&a, 20);
        let original = image::open(&a).unwrap().grayscale().to_luma8();
        imageops::rotate90(&original).save(&b).unwrap();

        let without = compare_images(&a, &b, 8, false).unwrap();
        let with = compare_images(&a, &b, 8, true).unwrap();
        assert!(with.distance <= without.distance);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn analysis_returns_hash_hex() {
        let root = temp_dir("analysis");
        let file = root.join("photo.png");
        sample_image(&file, 33);

        let analysis = analyze_image(&file, 8, true).unwrap();
        assert!(!analysis.perceptual_hash_hex.is_empty());
        assert_eq!(
            analysis.perceptual_hashes_hex[0],
            analysis.perceptual_hash_hex
        );
        assert!(analysis.transform_count >= 1);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn ffprobe_output_parser_extracts_duration_and_tags() {
        let parsed = parse_ffprobe_key_values(
            "duration=123.45\nTAG:title=Song A\nTAG:artist=Artist B\nTAG:album=Album C\n",
        );
        assert_eq!(parsed.duration_seconds, 123.45);
        assert_eq!(parsed.title.as_deref(), Some("Song A"));
        assert_eq!(parsed.artist.as_deref(), Some("Artist B"));
        assert_eq!(parsed.album.as_deref(), Some("Album C"));
    }

    #[test]
    fn video_and_audio_extension_detection_is_case_insensitive() {
        assert!(supported_video_extension(Path::new("clip.MP4")));
        assert!(supported_audio_extension(Path::new("song.FLAC")));
        assert!(!supported_video_extension(Path::new("note.txt")));
    }

    #[test]
    fn missing_media_tool_reports_clear_error() {
        let config = MediaToolConfig {
            ffmpeg_bin: "definitely-not-ffmpeg".to_string(),
            ffprobe_bin: "definitely-not-ffprobe".to_string(),
        };
        let err = media_tools_available(&config).unwrap_err();
        assert!(err
            .to_string()
            .contains("required dependency not available"));
    }
}
