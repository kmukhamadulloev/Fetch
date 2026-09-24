//! Managed FFmpeg exports. All command arguments are selected here, never by HTTP.
use std::{path::Path, process::Stdio, time::Duration};

use fetch_core::{ExportQuality, ExportRequest, FetchError, OutputFormat, ProcessingCapabilities};
use serde_json::Value;
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, BufReader},
    process::Command,
    sync::mpsc,
};
use tokio_util::sync::CancellationToken;

use crate::metadata_adapter::{Inspection, MetadataAdapter, io_error};

#[derive(Clone)]
pub struct ProcessingAdapter {
    pub media: MetadataAdapter,
}

fn invalid(message: &str) -> FetchError {
    FetchError::InvalidRequest(message.into())
}
fn cover(s: &Value) -> bool {
    s["disposition"]["attached_pic"] == 1
}
fn streams(i: &Inspection) -> &[Value] {
    i.raw["streams"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default()
}
fn video(i: &Inspection) -> Option<&Value> {
    streams(i)
        .iter()
        .find(|s| s["codec_type"] == "video" && !cover(s))
}
fn audio(i: &Inspection) -> Option<&Value> {
    streams(i).iter().find(|s| s["codec_type"] == "audio")
}
fn duration(i: &Inspection) -> Result<f64, FetchError> {
    i.raw["format"]["duration"]
        .as_str()
        .and_then(|s| s.parse::<f64>().ok())
        .filter(|v| v.is_finite() && *v > 0.)
        .ok_or_else(|| invalid("This media has no usable duration"))
}
fn dimensions(i: &Inspection) -> (Option<u32>, Option<u32>) {
    let Some(v) = video(i) else {
        return (None, None);
    };
    let mut w = v["width"].as_u64().and_then(|n| n.try_into().ok());
    let mut h = v["height"].as_u64().and_then(|n| n.try_into().ok());
    let rotation = v["side_data_list"]
        .as_array()
        .and_then(|a| a.iter().find_map(|s| s["rotation"].as_i64()))
        .unwrap_or(0);
    if rotation.rem_euclid(180) == 90 {
        std::mem::swap(&mut w, &mut h);
    }
    (w, h)
}
// Normalize non-square pixels before applying edits so displayed crop/resize
// coordinates and the resulting square-pixel export have the same aspect ratio.
fn export_dimensions(i: &Inspection) -> (Option<u32>, Option<u32>) {
    let (w, h) = dimensions(i);
    let Some(v) = video(i) else {
        return (w, h);
    };
    let ratio = v["sample_aspect_ratio"]
        .as_str()
        .and_then(|s| s.split_once(':'))
        .and_then(|(n, d)| Some((n.parse::<f64>().ok()?, d.parse::<f64>().ok()?)))
        .filter(|(n, d)| n.is_finite() && d.is_finite() && *n > 0. && *d > 0.)
        .map(|(n, d)| n / d)
        .unwrap_or(1.);
    let rotated = v["side_data_list"]
        .as_array()
        .and_then(|a| a.iter().find_map(|s| s["rotation"].as_i64()))
        .is_some_and(|r| r.rem_euclid(180) == 90);
    // Extend the original horizontal axis, even when orientation swaps axes.
    let scale = |value: Option<u32>| {
        value.map(|n| ((f64::from(n) * ratio / 2.).round().max(1.) * 2.) as u32)
    };
    if (ratio - 1.).abs() < 0.000001 {
        (w, h)
    } else if rotated {
        (w, scale(h))
    } else {
        (scale(w), h)
    }
}

fn copy_supported(i: &Inspection, f: OutputFormat) -> bool {
    use OutputFormat::*;
    let a = audio(i).and_then(|s| s["codec_name"].as_str());
    let v = video(i).and_then(|s| s["codec_name"].as_str());
    match f {
        Mp4 | Mkv => v == Some("h264") && (a.is_none() || a == Some("aac")),
        M4a => a == Some("aac"),
        Mp3 => a == Some("mp3"),
        Flac => a == Some("flac"),
        Wav => false,
    }
}

impl ProcessingAdapter {
    pub async fn capabilities(&self, path: &Path) -> Result<ProcessingCapabilities, FetchError> {
        let inspection = self.media.inspect(path).await?;
        self.capabilities_for(&inspection).await
    }
    async fn capabilities_for(&self, i: &Inspection) -> Result<ProcessingCapabilities, FetchError> {
        let encoders = self
            .media
            .run(
                self.media.paths.ffmpeg_executable(),
                &["-hide_banner".into(), "-encoders".into()],
                Duration::from_secs(15),
            )
            .await?;
        let encoders = String::from_utf8_lossy(&encoders);
        let has = |name: &str| {
            encoders
                .lines()
                .any(|line| line.split_whitespace().nth(1) == Some(name))
        };
        let formats = [
            OutputFormat::Mp4,
            OutputFormat::Mkv,
            OutputFormat::M4a,
            OutputFormat::Mp3,
            OutputFormat::Flac,
            OutputFormat::Wav,
        ]
        .into_iter()
        .filter(|f| {
            if f.video() {
                video(i).is_some() && has("libx264") && (audio(i).is_none() || has("aac"))
            } else {
                audio(i).is_some()
                    && has(match f {
                        OutputFormat::M4a => "aac",
                        OutputFormat::Mp3 => "libmp3lame",
                        OutputFormat::Flac => "flac",
                        _ => "pcm_s16le",
                    })
            }
        })
        .collect::<Vec<_>>();
        let copy_formats = [
            OutputFormat::Mp4,
            OutputFormat::Mkv,
            OutputFormat::M4a,
            OutputFormat::Mp3,
            OutputFormat::Flac,
        ]
        .into_iter()
        .filter(|f| copy_supported(i, *f))
        .collect();
        let (width, height) = export_dimensions(i);
        // Container-specific tag mappings are not universal; acknowledgement is explicit.
        let mut notices = vec!["container_metadata".into()];
        if streams(i).iter().filter(|s| !cover(s)).count()
            > usize::from(video(i).is_some()) + usize::from(audio(i).is_some())
        {
            notices.push("first_tracks_only".into());
        }
        if !i.covers.is_empty() || streams(i).iter().any(|s| s["codec_type"] == "attachment") {
            notices.push("artwork_compatibility".into());
        }
        if i.raw["chapters"].as_array().is_some_and(|c| !c.is_empty()) {
            notices.push("chapter_compatibility".into());
        }
        Ok(ProcessingCapabilities {
            revision: i.public.revision.clone(),
            duration_seconds: duration(i)?,
            video: video(i).is_some(),
            audio: audio(i).is_some(),
            width,
            height,
            formats,
            copy_formats,
            notices,
        })
    }

    pub async fn export(
        &self,
        input: &Path,
        output: &Path,
        request: &ExportRequest,
        cancel: &CancellationToken,
        progress: mpsc::Sender<f64>,
    ) -> Result<Inspection, FetchError> {
        request.validate()?;
        let inspection = self.media.inspect(input).await?;
        if inspection.public.revision != request.revision {
            return Err(crate::metadata::stale());
        }
        let caps = self.capabilities_for(&inspection).await?;
        let formats = if request.stream_copy {
            &caps.copy_formats
        } else {
            &caps.formats
        };
        if !formats.contains(&request.format) {
            return Err(invalid(
                "The installed runtime cannot export this source to the selected format",
            ));
        }
        if !request.acknowledge_omissions {
            return Err(invalid(
                "Review and acknowledge the stream and metadata limitations before exporting",
            ));
        }
        let plan = plan(input, output, &inspection, request)?;
        let mut command = Command::new(self.media.paths.ffmpeg_executable());
        command
            .args(&plan.args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        let mut child = command.spawn().map_err(io_error)?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| io_error("Missing progress stream"))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| io_error("Missing diagnostic stream"))?;
        let diagnostics = tokio::spawn(async move {
            let mut reader = BufReader::new(stderr);
            let mut chunk = [0u8; 4096];
            let mut tail = Vec::new();
            loop {
                let n = reader.read(&mut chunk).await?;
                if n == 0 {
                    break;
                }
                tail.extend_from_slice(&chunk[..n]);
                if tail.len() > 8192 {
                    tail.drain(..tail.len() - 8192);
                }
            }
            Ok::<_, std::io::Error>(tail)
        });
        let mut lines = BufReader::new(stdout).lines();
        let execution = async {
            while let Some(line) = lines.next_line().await.map_err(io_error)? {
                if let Some(value) = line
                    .strip_prefix("out_time_us=")
                    .and_then(|s| s.parse::<f64>().ok())
                    .filter(|v| v.is_finite())
                {
                    let _ = progress
                        .try_send((value / 1_000_000. / plan.duration * 100.).clamp(0., 99.));
                }
            }
            child.wait().await.map_err(io_error)
        };
        let result = tokio::select! {
            result=execution => result,
            _=cancel.cancelled() => Err(invalid("Export cancelled")),
            _=self.media.shutdown.cancelled() => Err(invalid("Export interrupted by shutdown")),
        };
        if result.is_err() {
            let _ = child.kill().await;
            let _ = child.wait().await;
        }
        let diagnostic = diagnostics.await.map_err(io_error)?.map_err(io_error)?;
        if !result?.success() {
            return Err(FetchError::ProcessFailed {summary:"The media runtime could not export this file. Check the format, permissions and free space.".into(),details:String::from_utf8_lossy(&diagnostic).into_owned()});
        }
        if cancel.is_cancelled() {
            return Err(invalid("Export cancelled"));
        }
        let output = self.media.inspect(output).await?;
        let actual = duration(&output)?;
        // Audio encoders add delay/padding; video cuts are bounded by frame duration.
        let fps = video(&inspection)
            .and_then(|v| v["avg_frame_rate"].as_str())
            .and_then(|s| s.split_once('/'))
            .and_then(|(n, d)| Some(n.parse::<f64>().ok()? / d.parse::<f64>().ok()?))
            .filter(|v| v.is_finite() && *v > 0.)
            .unwrap_or(25.);
        if (actual - plan.duration).abs() > 0.15 + 1. / fps
            || video(&output).is_some() != plan.video
            || audio(&output).is_some() != plan.audio
        {
            return Err(invalid(
                "Export validation failed: streams or duration do not match the requested output",
            ));
        }
        if let Some((w, h)) = plan.dimensions {
            let (ow, oh) = dimensions(&output);
            if ow != Some(w) || oh != Some(h) {
                return Err(invalid("Export validation failed: unexpected dimensions"));
            }
        }
        Ok(output)
    }
}

struct Plan {
    args: Vec<String>,
    duration: f64,
    video: bool,
    audio: bool,
    dimensions: Option<(u32, u32)>,
}
fn plan(
    input: &Path,
    output: &Path,
    i: &Inspection,
    r: &ExportRequest,
) -> Result<Plan, FetchError> {
    let total = duration(i)?;
    let e = r.edits.as_ref();
    let start = e.and_then(|e| e.start_seconds).unwrap_or(0.);
    let end = e.and_then(|e| e.end_seconds).unwrap_or(total);
    if start >= total || end > total + 0.001 || end <= start {
        return Err(invalid("Trim times must be inside the source duration"));
    }
    let has_video = r.format.video() && video(i).is_some();
    let has_audio = audio(i).is_some() && !e.is_some_and(|e| e.mute);
    if (!has_video && !has_audio) || (r.format.video() && !has_video) {
        return Err(invalid("No supported media stream was selected"));
    }
    if let Some(e) = e
        && ((!has_video && (e.rotate != 0 || e.crop.is_some() || e.resize.is_some() || e.mute))
            || (!has_audio && e.volume.is_some()))
    {
        return Err(invalid("These edits do not apply to the selected media"));
    }
    let mut args = vec![
        "-hide_banner".into(),
        "-loglevel".into(),
        "error".into(),
        "-nostdin".into(),
        "-n".into(),
        "-protocol_whitelist".into(),
        "file".into(),
        "-i".into(),
        input.to_string_lossy().into_owned(),
    ];
    let mut add = |key: &str, value: String| {
        args.push(key.into());
        args.push(value);
    };
    if start > 0. {
        add("-ss", start.to_string());
    }
    if end < total || start > 0. {
        add("-t", (end - start).to_string());
    }
    if has_video {
        add("-map", format!("0:{}", video(i).unwrap()["index"]));
    }
    if has_audio {
        add("-map", format!("0:{}", audio(i).unwrap()["index"]));
    }
    let artwork = streams(i)
        .iter()
        .find(|s| cover(s) && matches!(s["codec_name"].as_str(), Some("mjpeg" | "png")));
    let keep_cover = artwork.is_some()
        && matches!(
            r.format,
            OutputFormat::Mp4 | OutputFormat::M4a | OutputFormat::Mp3 | OutputFormat::Flac
        );
    if keep_cover {
        add("-map", format!("0:{}", artwork.unwrap()["index"]));
    }
    add("-map_metadata", "0".into());
    add(
        "-map_chapters",
        if r.format.video() && start == 0. && end == total {
            "0"
        } else {
            "-1"
        }
        .into(),
    );
    let mut dims = if has_video {
        let (w, h) = if r.stream_copy {
            dimensions(i)
        } else {
            export_dimensions(i)
        };
        Some((
            w.ok_or_else(|| invalid("Missing video dimensions"))?,
            h.ok_or_else(|| invalid("Missing video dimensions"))?,
        ))
    } else {
        None
    };
    if r.stream_copy {
        add("-c", "copy".into());
    } else {
        let (crf, bitrate) = match r.quality {
            ExportQuality::Compact => ("28", "128k"),
            ExportQuality::Balanced => ("23", "192k"),
            ExportQuality::High => (
                "18",
                if r.format == OutputFormat::Mp3 {
                    "320k"
                } else {
                    "256k"
                },
            ),
        };
        if has_video {
            add("-c:v:0", "libx264".into());
            add("-crf", crf.into());
            add("-preset", "medium".into());
            add("-pix_fmt", "yuv420p".into());
            let (mut w, mut h) = dims.unwrap();
            let mut filters = Vec::new();
            if dimensions(i) != export_dimensions(i) {
                if w > 7680 || h > 7680 {
                    return Err(invalid("Normalized video dimensions exceed 7680 pixels"));
                }
                filters.push(format!("scale={w}:{h},setsar=1"));
            }
            if let Some(e) = e {
                if let Some(c) = &e.crop {
                    if c.x.checked_add(c.width).is_none_or(|n| n > w)
                        || c.y.checked_add(c.height).is_none_or(|n| n > h)
                        || c.x % 2 != 0
                        || c.y % 2 != 0
                        || c.width % 2 != 0
                        || c.height % 2 != 0
                    {
                        return Err(invalid(
                            "Crop must use even dimensions and fit inside the source",
                        ));
                    }
                    filters.push(format!("crop={}:{}:{}:{}", c.width, c.height, c.x, c.y));
                    w = c.width;
                    h = c.height;
                }
                match e.rotate {
                    90 => {
                        filters.push("transpose=clock".into());
                        std::mem::swap(&mut w, &mut h);
                    }
                    180 => filters.push("hflip,vflip".into()),
                    270 => {
                        filters.push("transpose=cclock".into());
                        std::mem::swap(&mut w, &mut h);
                    }
                    _ => {}
                }
                if let Some(s) = &e.resize {
                    filters.push(format!("scale={}:{}", s.width, s.height));
                    w = s.width;
                    h = s.height;
                }
            }
            if w == 0 || h == 0 || w % 2 != 0 || h % 2 != 0 || w > 7680 || h > 7680 {
                return Err(invalid(
                    "H.264 export requires even dimensions up to 7680 pixels",
                ));
            }
            dims = Some((w, h));
            filters.push("setsar=1".into());
            add("-filter:v:0", filters.join(","));
            add("-metadata:s:v:0", "rotate=0".into());
        }
        if has_audio {
            add(
                "-c:a",
                match r.format {
                    OutputFormat::Mp3 => "libmp3lame",
                    OutputFormat::Flac => "flac",
                    OutputFormat::Wav => "pcm_s16le",
                    _ => "aac",
                }
                .into(),
            );
            if !matches!(r.format, OutputFormat::Flac | OutputFormat::Wav) {
                add("-b:a", bitrate.into());
            }
            if let Some(volume) = e.and_then(|e| e.volume) {
                add("-af", format!("volume={volume}"));
            }
        }
        if keep_cover {
            let index = usize::from(has_video);
            add(&format!("-c:v:{index}"), "copy".into());
        }
    }
    if keep_cover {
        add(
            &format!("-disposition:v:{}", usize::from(has_video)),
            "attached_pic".into(),
        );
    }
    if matches!(r.format, OutputFormat::Mp4 | OutputFormat::M4a) {
        add("-movflags", "+faststart".into());
    }
    add("-progress", "pipe:1".into());
    add("-stats_period", "0.25".into());
    args.push(output.to_string_lossy().into_owned());
    Ok(Plan {
        args,
        duration: end - start,
        video: has_video,
        audio: has_audio,
        dimensions: dims,
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use fetch_core::{Crop, QuickEdits, Resize};
    use fetch_runtime::RuntimePaths;
    pub(crate) fn real_adapter() -> ProcessingAdapter {
        ProcessingAdapter {
            media: MetadataAdapter {
                paths: RuntimePaths::new(
                    std::env::var_os("FETCH_METADATA_RUNTIME")
                        .map(std::path::PathBuf::from)
                        .unwrap_or_else(|| "/tmp/fetch-metadata-runtime".into()),
                ),
                shutdown: CancellationToken::new(),
            },
        }
    }
    pub(crate) async fn source(adapter: &ProcessingAdapter, path: &Path) {
        let args = [
            "-v",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "testsrc2=size=96x64:rate=25:duration=3",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=3",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
            "-metadata",
            "title=Keep this title",
        ]
        .into_iter()
        .map(str::to_owned)
        .chain([path.to_string_lossy().into_owned()])
        .collect::<Vec<_>>();
        adapter
            .media
            .run(
                adapter.media.paths.ffmpeg_executable(),
                &args,
                Duration::from_secs(20),
            )
            .await
            .unwrap();
    }
    pub(crate) async fn request(adapter: &ProcessingAdapter, path: &Path) -> ExportRequest {
        ExportRequest {
            revision: adapter.media.inspect(path).await.unwrap().public.revision,
            filename: "Export".into(),
            format: OutputFormat::Mp4,
            quality: ExportQuality::Balanced,
            stream_copy: false,
            edits: None,
            acknowledge_omissions: true,
        }
    }
    #[tokio::test]
    #[ignore = "requires installed managed FFmpeg/FFprobe"]
    async fn real_managed_processing_orientation_and_pixel_aspect() {
        let adapter = real_adapter();
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("source.mp4");
        source(&adapter, &input).await;
        for rotated in [false, true] {
            let anamorphic = dir.path().join(format!("anamorphic-{rotated}.mp4"));
            adapter
                .media
                .run(
                    adapter.media.paths.ffmpeg_executable(),
                    &[
                        "-v".into(),
                        "error".into(),
                        "-i".into(),
                        input.to_string_lossy().into_owned(),
                        "-vf".into(),
                        "setsar=2/1".into(),
                        "-c:v".into(),
                        "libx264".into(),
                        "-c:a".into(),
                        "copy".into(),
                        anamorphic.to_string_lossy().into_owned(),
                    ],
                    Duration::from_secs(20),
                )
                .await
                .unwrap();
            let source = if rotated {
                let path = dir.path().join("oriented.mp4");
                adapter
                    .media
                    .run(
                        adapter.media.paths.ffmpeg_executable(),
                        &[
                            "-v".into(),
                            "error".into(),
                            "-display_rotation:v:0".into(),
                            "90".into(),
                            "-i".into(),
                            anamorphic.to_string_lossy().into_owned(),
                            "-c".into(),
                            "copy".into(),
                            path.to_string_lossy().into_owned(),
                        ],
                        Duration::from_secs(20),
                    )
                    .await
                    .unwrap();
                path
            } else {
                anamorphic
            };
            let original = tokio::fs::read(&source).await.unwrap();
            let caps = adapter.capabilities(&source).await.unwrap();
            let expected = if rotated {
                (Some(64), Some(192))
            } else {
                (Some(192), Some(64))
            };
            assert_eq!((caps.width, caps.height), expected);
            let r = request(&adapter, &source).await;
            let output = dir.path().join(format!("normalized-{rotated}.mp4"));
            let (tx, _rx) = mpsc::channel(128);
            let result = adapter
                .export(&source, &output, &r, &CancellationToken::new(), tx)
                .await
                .unwrap();
            assert_eq!(dimensions(&result), expected);
            assert_eq!(video(&result).unwrap()["sample_aspect_ratio"], "1:1");
            assert_eq!(tokio::fs::read(&source).await.unwrap(), original);
        }
    }

    #[tokio::test]
    #[ignore = "requires installed managed FFmpeg/FFprobe"]
    async fn real_managed_processing_artwork_chapters_and_volume() {
        let adapter = real_adapter();
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("source.mp4");
        source(&adapter, &input).await;
        let cover_path = dir.path().join("cover.png");
        let metadata = dir.path().join("chapters.txt");
        tokio::fs::write(&metadata, ";FFMETADATA1\ntitle=Keep this title\n[CHAPTER]\nTIMEBASE=1/1000\nSTART=0\nEND=3000\ntitle=Chapter one\n").await.unwrap();
        adapter
            .media
            .run(
                adapter.media.paths.ffmpeg_executable(),
                &[
                    "-v".into(),
                    "error".into(),
                    "-f".into(),
                    "lavfi".into(),
                    "-i".into(),
                    "color=red:size=32x32".into(),
                    "-frames:v".into(),
                    "1".into(),
                    cover_path.to_string_lossy().into_owned(),
                ],
                Duration::from_secs(20),
            )
            .await
            .unwrap();
        let decorated = dir.path().join("decorated.mp4");
        adapter
            .media
            .run(
                adapter.media.paths.ffmpeg_executable(),
                &[
                    "-v".into(),
                    "error".into(),
                    "-i".into(),
                    input.to_string_lossy().into_owned(),
                    "-i".into(),
                    cover_path.to_string_lossy().into_owned(),
                    "-i".into(),
                    metadata.to_string_lossy().into_owned(),
                    "-map".into(),
                    "0:v:0".into(),
                    "-map".into(),
                    "0:a:0".into(),
                    "-map".into(),
                    "1:v:0".into(),
                    "-map_metadata".into(),
                    "2".into(),
                    "-map_chapters".into(),
                    "2".into(),
                    "-c".into(),
                    "copy".into(),
                    "-disposition:v:1".into(),
                    "attached_pic".into(),
                    decorated.to_string_lossy().into_owned(),
                ],
                Duration::from_secs(20),
            )
            .await
            .unwrap();
        let original = tokio::fs::read(&decorated).await.unwrap();
        for format in [
            OutputFormat::Mp4,
            OutputFormat::Mkv,
            OutputFormat::M4a,
            OutputFormat::Mp3,
            OutputFormat::Flac,
            OutputFormat::Wav,
        ] {
            let mut r = request(&adapter, &decorated).await;
            r.format = format;
            let output = dir.path().join(format!("tags.{}", format.extension()));
            let (tx, _rx) = mpsc::channel(128);
            let result = adapter
                .export(&decorated, &output, &r, &CancellationToken::new(), tx)
                .await
                .unwrap();
            assert_eq!(
                streams(&result).iter().any(cover),
                !matches!(format, OutputFormat::Mkv | OutputFormat::Wav)
            );
            assert_eq!(
                result.raw["chapters"]
                    .as_array()
                    .is_some_and(|c| !c.is_empty()),
                format.video()
            );
        }
        async fn rms(adapter: &ProcessingAdapter, path: &Path) -> f64 {
            let pcm = adapter
                .media
                .run(
                    adapter.media.paths.ffmpeg_executable(),
                    &[
                        "-v".into(),
                        "error".into(),
                        "-i".into(),
                        path.to_string_lossy().into_owned(),
                        "-map".into(),
                        "0:a:0".into(),
                        "-f".into(),
                        "f32le".into(),
                        "-".into(),
                    ],
                    Duration::from_secs(20),
                )
                .await
                .unwrap();
            let count = pcm.len() / 4;
            (pcm.as_chunks::<4>()
                .0
                .iter()
                .map(|bytes| f64::from(f32::from_le_bytes(*bytes)).powi(2))
                .sum::<f64>()
                / count as f64)
                .sqrt()
        }
        let mut r = request(&adapter, &decorated).await;
        r.format = OutputFormat::Wav;
        r.edits = Some(QuickEdits {
            volume: Some(0.5),
            ..Default::default()
        });
        let output = dir.path().join("quiet.wav");
        let (tx, _rx) = mpsc::channel(128);
        adapter
            .export(&decorated, &output, &r, &CancellationToken::new(), tx)
            .await
            .unwrap();
        let ratio = rms(&adapter, &output).await / rms(&adapter, &decorated).await;
        assert!((ratio - 0.5).abs() < 0.01, "volume ratio {ratio}");
        assert_eq!(tokio::fs::read(&decorated).await.unwrap(), original);
    }

    async fn packet_hash(adapter: &ProcessingAdapter, path: &Path) -> Vec<u8> {
        adapter
            .media
            .run(
                adapter.media.paths.ffmpeg_executable(),
                &[
                    "-v".into(),
                    "error".into(),
                    "-i".into(),
                    path.to_string_lossy().into_owned(),
                    "-map".into(),
                    "0".into(),
                    "-c".into(),
                    "copy".into(),
                    "-f".into(),
                    "streamhash".into(),
                    "-hash".into(),
                    "sha256".into(),
                    "-".into(),
                ],
                Duration::from_secs(10),
            )
            .await
            .unwrap()
    }
    #[tokio::test]
    #[ignore = "requires installed managed FFmpeg/FFprobe"]
    async fn real_managed_processing_matrix_and_combined_edits() {
        let adapter = real_adapter();
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("source.mp4");
        source(&adapter, &input).await;
        let original = tokio::fs::read(&input).await.unwrap();
        let caps = adapter.capabilities(&input).await.unwrap();
        assert_eq!(caps.formats.len(), 6);
        for format in &caps.formats {
            let mut r = request(&adapter, &input).await;
            r.format = *format;
            let output = dir.path().join(format!("converted.{}", format.extension()));
            let (tx, _rx) = mpsc::channel(128);
            let inspected = adapter
                .export(&input, &output, &r, &CancellationToken::new(), tx)
                .await
                .unwrap();
            assert!(audio(&inspected).is_some());
            assert_eq!(video(&inspected).is_some(), format.video());
            if *format != OutputFormat::Wav {
                assert_eq!(
                    inspected.raw["format"]["tags"]["title"]
                        .as_str()
                        .or_else(|| inspected.raw["format"]["tags"]["TITLE"].as_str()),
                    Some("Keep this title")
                );
            }
        }
        for format in [OutputFormat::Mp4, OutputFormat::Mkv] {
            let mut r = request(&adapter, &input).await;
            r.format = format;
            r.stream_copy = true;
            let output = dir.path().join(format!("copy.{}", format.extension()));
            let (tx, _rx) = mpsc::channel(128);
            adapter
                .export(&input, &output, &r, &CancellationToken::new(), tx)
                .await
                .unwrap();
            assert_eq!(
                packet_hash(&adapter, &input).await,
                packet_hash(&adapter, &output).await
            );
        }
        for (n, edits) in [
            QuickEdits {
                start_seconds: Some(0.4),
                end_seconds: Some(2.),
                ..Default::default()
            },
            QuickEdits {
                rotate: 90,
                ..Default::default()
            },
            QuickEdits {
                mute: true,
                ..Default::default()
            },
            QuickEdits {
                crop: Some(Crop {
                    x: 8,
                    y: 8,
                    width: 80,
                    height: 48,
                }),
                ..Default::default()
            },
            QuickEdits {
                resize: Some(Resize {
                    width: 48,
                    height: 32,
                }),
                ..Default::default()
            },
            QuickEdits {
                volume: Some(0.5),
                ..Default::default()
            },
            QuickEdits {
                start_seconds: Some(0.4),
                end_seconds: Some(2.),
                rotate: 90,
                crop: Some(Crop {
                    x: 8,
                    y: 8,
                    width: 80,
                    height: 48,
                }),
                resize: Some(Resize {
                    width: 24,
                    height: 40,
                }),
                volume: Some(0.5),
                mute: false,
            },
        ]
        .into_iter()
        .enumerate()
        {
            let mut r = request(&adapter, &input).await;
            r.edits = Some(edits);
            let output = dir.path().join(format!("edit-{n}.mp4"));
            let (tx, _rx) = mpsc::channel(128);
            let result = adapter
                .export(&input, &output, &r, &CancellationToken::new(), tx)
                .await
                .unwrap();
            if n == 2 {
                assert!(audio(&result).is_none());
            }
            if n == 6 {
                assert_eq!(dimensions(&result), (Some(24), Some(40)));
                let v = video(&result).unwrap()["start_time"]
                    .as_str()
                    .unwrap()
                    .parse::<f64>()
                    .unwrap();
                let a = audio(&result).unwrap()["start_time"]
                    .as_str()
                    .unwrap()
                    .parse::<f64>()
                    .unwrap();
                assert!((v - a).abs() < 0.08);
            }
        }
        assert_eq!(tokio::fs::read(&input).await.unwrap(), original);
        let mut r = request(&adapter, &input).await;
        r.edits = Some(QuickEdits {
            crop: Some(Crop {
                x: 100,
                y: 0,
                width: 64,
                height: 32,
            }),
            ..Default::default()
        });
        let (tx, _rx) = mpsc::channel(8);
        assert!(
            adapter
                .export(
                    &input,
                    &dir.path().join("invalid.mp4"),
                    &r,
                    &CancellationToken::new(),
                    tx
                )
                .await
                .is_err()
        );
        assert!(!dir.path().join("invalid.mp4").exists());
    }
}
