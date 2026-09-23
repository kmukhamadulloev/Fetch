//! Managed process adapter. Never receives client filesystem paths or command arguments.
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    time::Duration,
};

use base64::Engine;
use fetch_core::{ArtworkUpdate, FetchError, METADATA_FIELDS, MediaMetadata, MetadataUpdate};
use fetch_runtime::RuntimePaths;
use serde_json::Value;
use tokio::{io::AsyncReadExt, process::Command};
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub struct MetadataAdapter {
    pub paths: RuntimePaths,
    pub shutdown: CancellationToken,
}

pub struct Inspection {
    pub public: MediaMetadata,
    pub raw: Value,
    pub covers: Vec<usize>,
}

pub fn io_error(error: impl std::fmt::Display) -> FetchError {
    FetchError::ProcessFailed {
        summary: "Could not read or write media. Check permissions and available disk space."
            .into(),
        details: error.to_string(),
    }
}

pub async fn revision(path: &Path) -> Result<String, FetchError> {
    let metadata = tokio::fs::symlink_metadata(path).await.map_err(io_error)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(FetchError::InvalidRequest(
            "Only regular completed files can be edited".into(),
        ));
    }
    let modified = metadata
        .modified()
        .map_err(io_error)?
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(io_error)?;
    Ok(format!("{}-{}", metadata.len(), modified.as_nanos()))
}

fn tags(value: &Value) -> BTreeMap<String, String> {
    value
        .as_object()
        .map(|object| {
            object
                .iter()
                .filter_map(|(key, value)| {
                    value
                        .as_str()
                        .map(|value| (key.to_lowercase(), value.to_owned()))
                })
                .collect()
        })
        .unwrap_or_default()
}

impl MetadataAdapter {
    pub(crate) async fn run(
        &self,
        executable: PathBuf,
        args: &[String],
        timeout: Duration,
    ) -> Result<Vec<u8>, FetchError> {
        if !executable.is_file() {
            return Err(FetchError::RuntimeMissing(
                executable
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into(),
            ));
        }
        let mut command = Command::new(executable);
        command
            .args(args)
            .kill_on_drop(true)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());
        let mut child = command.spawn().map_err(io_error)?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| io_error("Missing runtime stdout"))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| io_error("Missing runtime stderr"))?;
        let execution = async {
            let (status, stdout, stderr) = tokio::try_join!(
                async { child.wait().await.map_err(io_error) },
                bounded_read(stdout, 16 * 1024 * 1024),
                bounded_read(stderr, 1024 * 1024)
            )?;
            if !status.success() {
                return Err(FetchError::ProcessFailed {
                    summary: "The media runtime could not safely process this file".into(),
                    details: String::from_utf8_lossy(&stderr)
                        .chars()
                        .take(4096)
                        .collect(),
                });
            }
            Ok(stdout)
        };
        tokio::select! {
            _ = self.shutdown.cancelled() => Err(FetchError::InvalidRequest("Metadata save interrupted by shutdown".into())),
            result = tokio::time::timeout(timeout, execution) => result.map_err(|_| FetchError::ProcessFailed { summary: "Media operation timed out".into(), details: String::new() })?,
        }
    }

    pub async fn inspect(&self, path: &Path) -> Result<Inspection, FetchError> {
        let revision = revision(path).await?;
        let bytes = self
            .run(
                self.paths.ffprobe_executable(),
                &[
                    "-v".into(),
                    "error".into(),
                    "-protocol_whitelist".into(),
                    "file".into(),
                    "-show_format".into(),
                    "-show_streams".into(),
                    "-show_chapters".into(),
                    "-of".into(),
                    "json".into(),
                    path.to_string_lossy().into(),
                ],
                Duration::from_secs(30),
            )
            .await?;
        let raw: Value = serde_json::from_slice(&bytes).map_err(io_error)?;
        let streams = raw["streams"]
            .as_array()
            .ok_or_else(|| io_error("Missing media streams"))?;
        let covers = streams
            .iter()
            .filter(|stream| is_cover(stream))
            .filter_map(|stream| stream["index"].as_u64().map(|index| index as usize))
            .collect::<Vec<_>>();
        let video = streams
            .iter()
            .any(|stream| stream["codec_type"] == "video" && !is_cover(stream));
        let extension = path
            .extension()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase();
        let detected = raw["format"]["format_name"].as_str().unwrap_or_default();
        let editable = match extension.as_str() {
            "mp3" => detected == "mp3",
            "flac" => detected == "flac",
            "m4a" | "mp4" => detected.split(',').any(|name| name == "mp4"),
            "mkv" => detected.split(',').any(|name| name == "matroska"),
            _ => false,
        };
        let all_tags = tags(&raw["format"]["tags"]);
        let mut fields = BTreeMap::new();
        let supported_fields: Vec<String> = if editable {
            METADATA_FIELDS
                .iter()
                .filter(|key| !(extension == "flac" && **key == "description"))
                .filter(|key| {
                    !video
                        || [
                            "title",
                            "artist",
                            "date",
                            "genre",
                            "copyright",
                            "comment",
                            "description",
                        ]
                        .contains(key)
                })
                .map(|key| (*key).into())
                .collect()
        } else {
            vec![]
        };
        for key in &supported_fields {
            let value = all_tags.get(key).cloned().unwrap_or_default();
            fields.insert(key.clone(), value);
        }
        for key in ["track", "disc"] {
            let value = fields.get(key).cloned().unwrap_or_default();
            if let Some((number, total)) = value.split_once('/') {
                fields.insert(key.into(), number.into());
                fields.insert(format!("{key}_total"), total.into());
            }
        }
        let mut information = BTreeMap::new();
        information.insert(
            "filename".into(),
            path.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into(),
        );
        for key in ["duration", "size", "bit_rate"] {
            if let Some(value) = raw["format"][key].as_str() {
                information.insert(key.into(), value.into());
            }
        }
        for (index, stream) in streams
            .iter()
            .filter(|stream| !is_cover(stream))
            .enumerate()
        {
            let value = [
                "codec_type",
                "codec_name",
                "width",
                "height",
                "r_frame_rate",
                "sample_rate",
                "channels",
            ]
            .iter()
            .filter_map(|key| {
                let value = &stream[key];
                (!value.is_null()).then(|| {
                    format!(
                        "{key}: {}",
                        value
                            .as_str()
                            .map(str::to_owned)
                            .unwrap_or_else(|| value.to_string())
                    )
                })
            })
            .collect::<Vec<_>>()
            .join(" · ");
            information.insert(format!("stream_{}", index + 1), value);
        }
        Ok(Inspection {
            public: MediaMetadata {
                revision,
                editable,
                media_type: if video { "video" } else { "audio" }.into(),
                container: extension,
                fields,
                supported_fields,
                artwork_available: !covers.is_empty(),
                artwork_editable: editable,
                information,
            },
            raw,
            covers,
        })
    }

    pub async fn artwork(
        &self,
        path: &Path,
        inspection: &Inspection,
    ) -> Result<Vec<u8>, FetchError> {
        let cover = inspection.covers.first().ok_or(FetchError::FileNotFound)?;
        self.run(
            self.paths.ffmpeg_executable(),
            &[
                "-v".into(),
                "error".into(),
                "-nostdin".into(),
                "-protocol_whitelist".into(),
                "file,pipe".into(),
                "-i".into(),
                path.to_string_lossy().into(),
                "-map".into(),
                format!("0:{cover}"),
                "-frames:v".into(),
                "1".into(),
                "-vf".into(),
                "scale=1024:1024:force_original_aspect_ratio=decrease".into(),
                "-f".into(),
                "image2pipe".into(),
                "-c:v".into(),
                "mjpeg".into(),
                "pipe:1".into(),
            ],
            Duration::from_secs(30),
        )
        .await
    }

    pub async fn prepare_artwork(&self, data: &str, target: &Path) -> Result<(), FetchError> {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(data)
            .map_err(|_| FetchError::InvalidRequest("Invalid artwork encoding".into()))?;
        if bytes.len() > 8 * 1024 * 1024 {
            return Err(FetchError::InvalidRequest(
                "Artwork must be at most 8 MiB".into(),
            ));
        }
        let target = target.to_owned();
        tokio::task::spawn_blocking(move || {
            let format = image::guess_format(&bytes)
                .map_err(|_| FetchError::InvalidRequest("Artwork must be JPEG or PNG".into()))?;
            if !matches!(format, image::ImageFormat::Jpeg | image::ImageFormat::Png) {
                return Err(FetchError::InvalidRequest(
                    "Artwork must be JPEG or PNG".into(),
                ));
            }
            let mut reader = image::ImageReader::with_format(std::io::Cursor::new(bytes), format);
            let mut limits = image::Limits::default();
            limits.max_image_width = Some(8192);
            limits.max_image_height = Some(8192);
            limits.max_alloc = Some(128 * 1024 * 1024);
            reader.limits(limits);
            let image = reader.decode().map_err(|_| {
                FetchError::InvalidRequest(
                    "Artwork is invalid or exceeds image dimensions/memory limits".into(),
                )
            })?;
            image
                .thumbnail(2048, 2048)
                .to_rgb8()
                .save_with_format(target, image::ImageFormat::Jpeg)
                .map_err(io_error)
        })
        .await
        .map_err(io_error)?
    }

    pub async fn write(
        &self,
        source: &Path,
        target: &Path,
        inspection: &Inspection,
        update: &MetadataUpdate,
        artwork: Option<&Path>,
    ) -> Result<Inspection, FetchError> {
        update.validate()?;
        if !inspection.public.editable
            || update
                .fields
                .keys()
                .any(|key| !inspection.public.supported_fields.contains(key))
        {
            return Err(FetchError::InvalidRequest(
                "This file or metadata field is not supported for editing".into(),
            ));
        }
        // Matroska demuxes image attachments as attached-picture video streams.
        // Mapping those directly would turn them into timed video: extract their
        // compressed bytes and reattach them instead.
        let scratch = target.with_extension("cover");
        let keep_mkv = inspection.public.container == "mkv"
            && matches!(update.artwork, ArtworkUpdate::Keep)
            && !inspection.covers.is_empty();
        if keep_mkv && inspection.covers.len() != 1 {
            return Err(FetchError::InvalidRequest(
                "Editing Matroska files with multiple cover attachments is not supported".into(),
            ));
        }
        if keep_mkv {
            let bytes = self
                .run(
                    self.paths.ffmpeg_executable(),
                    &[
                        "-v".into(),
                        "error".into(),
                        "-nostdin".into(),
                        "-protocol_whitelist".into(),
                        "file,pipe".into(),
                        "-i".into(),
                        source.to_string_lossy().into(),
                        "-map".into(),
                        format!("0:{}", inspection.covers[0]),
                        "-c".into(),
                        "copy".into(),
                        "-frames:v".into(),
                        "1".into(),
                        "-f".into(),
                        "image2pipe".into(),
                        "pipe:1".into(),
                    ],
                    Duration::from_secs(30),
                )
                .await?;
            tokio::fs::write(&scratch, bytes).await.map_err(io_error)?;
        }
        let artwork = if keep_mkv {
            Some(scratch.as_path())
        } else {
            artwork
        };
        let mut args = vec![
            "-v".into(),
            "error".into(),
            "-nostdin".into(),
            "-n".into(),
            "-protocol_whitelist".into(),
            "file".into(),
            "-i".into(),
            source.to_string_lossy().into(),
        ];
        let mkv = inspection.public.container == "mkv";
        if let Some(artwork) = artwork.filter(|_| !mkv) {
            args.extend(["-i".into(), artwork.to_string_lossy().into()]);
        }
        args.extend(["-map".into(), "0".into()]);
        if !matches!(update.artwork, ArtworkUpdate::Keep) || keep_mkv {
            for index in &inspection.covers {
                args.extend(["-map".into(), format!("-0:{index}")]);
            }
        }
        let original_streams = inspection.raw["streams"].as_array().unwrap();
        if let Some(artwork) = artwork {
            if mkv {
                let index = original_streams
                    .iter()
                    .filter(|stream| !is_cover(stream))
                    .count();
                args.extend([
                    "-attach".into(),
                    artwork.to_string_lossy().into(),
                    format!("-metadata:s:{index}"),
                    if keep_mkv {
                        format!(
                            "mimetype={}",
                            inspection.raw["streams"][inspection.covers[0]]["tags"]["mimetype"]
                                .as_str()
                                .unwrap_or("image/jpeg")
                        )
                    } else {
                        "mimetype=image/jpeg".into()
                    },
                    format!("-metadata:s:{index}"),
                    if keep_mkv {
                        format!(
                            "filename={}",
                            inspection.raw["streams"][inspection.covers[0]]["tags"]["filename"]
                                .as_str()
                                .unwrap_or("cover.jpg")
                        )
                    } else {
                        "filename=cover.jpg".into()
                    },
                ]);
            } else {
                let index = original_streams
                    .iter()
                    .filter(|stream| stream["codec_type"] == "video" && !is_cover(stream))
                    .count();
                args.extend([
                    "-map".into(),
                    "1:v:0".into(),
                    format!("-disposition:v:{index}"),
                    "attached_pic".into(),
                    format!("-metadata:s:v:{index}"),
                    "title=Album cover".into(),
                    format!("-metadata:s:v:{index}"),
                    "comment=Cover (front)".into(),
                ]);
            }
        }
        args.extend([
            "-c".into(),
            "copy".into(),
            "-map_metadata".into(),
            "0".into(),
            "-map_chapters".into(),
            "0".into(),
        ]);
        let mut changed = update.fields.clone();
        for key in ["track", "disc"] {
            let total_key = format!("{key}_total");
            if changed.contains_key(key) || changed.contains_key(&total_key) {
                let number = changed
                    .get(key)
                    .or_else(|| inspection.public.fields.get(key))
                    .cloned()
                    .unwrap_or_default();
                let total = changed
                    .remove(&total_key)
                    .or_else(|| inspection.public.fields.get(&total_key).cloned())
                    .unwrap_or_default();
                if number.is_empty() && !total.is_empty() {
                    return Err(FetchError::InvalidRequest(
                        "A total requires a track or disc number".into(),
                    ));
                }
                if let (Ok(number), Ok(total)) = (number.parse::<u16>(), total.parse::<u16>())
                    && number > total
                {
                    return Err(FetchError::InvalidRequest(
                        "Track or disc number exceeds its total".into(),
                    ));
                }
                changed.insert(
                    key.into(),
                    if total.is_empty() {
                        number
                    } else {
                        format!("{number}/{total}")
                    },
                );
            }
        }
        for (key, value) in &changed {
            args.extend(["-metadata".into(), format!("{key}={value}")]);
        }
        args.push(target.to_string_lossy().into());
        self.run(
            self.paths.ffmpeg_executable(),
            &args,
            Duration::from_secs(3600),
        )
        .await?;
        let result = self.inspect(target).await?;
        for (key, value) in &update.fields {
            if result
                .public
                .fields
                .get(key)
                .map(String::as_str)
                .unwrap_or_default()
                != value
            {
                return Err(FetchError::InvalidRequest(format!(
                    "The container could not preserve the requested {key} tag"
                )));
            }
        }
        if matches!(update.artwork, ArtworkUpdate::Keep)
            && inspection.covers.len() != result.covers.len()
        {
            return Err(FetchError::InvalidRequest(
                "Cannot preserve existing artwork".into(),
            ));
        }
        let before_streams = inspection.raw["streams"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|stream| !is_cover(stream));
        let after_streams = result.raw["streams"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|stream| !is_cover(stream));
        for (before, after) in before_streams.zip(after_streams) {
            let after_tags = tags(&after["tags"]);
            for (key, value) in tags(&before["tags"]) {
                if key != "duration" && after_tags.get(&key) != Some(&value) {
                    return Err(FetchError::InvalidRequest(format!(
                        "Cannot preserve existing stream {key} metadata"
                    )));
                }
            }
            if before["disposition"] != after["disposition"] {
                return Err(FetchError::InvalidRequest(
                    "Cannot preserve stream dispositions".into(),
                ));
            }
        }
        let duration = |raw: &Value| {
            raw["format"]["duration"]
                .as_str()
                .and_then(|value| value.parse::<f64>().ok())
        };
        if let (Some(before), Some(after)) = (duration(&inspection.raw), duration(&result.raw))
            && (before - after).abs() > 0.25
        {
            return Err(FetchError::InvalidRequest(
                "Cannot preserve media duration".into(),
            ));
        }
        let before = stream_signatures(&inspection.raw);
        let after = stream_signatures(&result.raw);
        if before != after || !chapters_match(&inspection.raw, &result.raw) {
            return Err(FetchError::InvalidRequest(
                "This file cannot be safely remuxed without changing streams or chapters".into(),
            ));
        }
        if matches!(update.artwork, ArtworkUpdate::Replace { .. }) && result.covers.len() != 1
            || matches!(update.artwork, ArtworkUpdate::Remove) && !result.covers.is_empty()
        {
            return Err(FetchError::InvalidRequest(
                "The container could not save the requested artwork".into(),
            ));
        }
        // Unknown container tags must survive; muxer-generated bookkeeping may change.
        let after_tags = tags(&result.raw["format"]["tags"]);
        for (key, value) in tags(&inspection.raw["format"]["tags"]) {
            if !changed.contains_key(&key)
                && ![
                    "encoder",
                    "major_brand",
                    "minor_version",
                    "compatible_brands",
                ]
                .contains(&key.as_str())
                && after_tags.get(&key) != Some(&value)
            {
                return Err(FetchError::InvalidRequest(format!(
                    "Cannot preserve existing {key} metadata in this container"
                )));
            }
        }
        if keep_mkv {
            tokio::fs::remove_file(&scratch).await.map_err(io_error)?;
        }
        Ok(result)
    }
}

async fn bounded_read(
    reader: impl tokio::io::AsyncRead + Unpin,
    limit: u64,
) -> Result<Vec<u8>, FetchError> {
    let mut bytes = Vec::new();
    reader
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .await
        .map_err(io_error)?;
    if bytes.len() as u64 > limit {
        return Err(FetchError::InvalidRequest(
            "Media runtime output exceeds inspection limits".into(),
        ));
    }
    Ok(bytes)
}

fn is_cover(stream: &Value) -> bool {
    stream["disposition"]["attached_pic"] == 1
        || (stream["codec_type"] == "attachment"
            && stream["tags"]["filename"].as_str().is_some_and(|name| {
                matches!(
                    name.to_lowercase().as_str(),
                    "cover.jpg" | "cover.jpeg" | "cover.png"
                )
            }))
}

fn stream_signatures(raw: &Value) -> Vec<String> {
    raw["streams"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|stream| !is_cover(stream))
        .map(|stream| {
            [
                "codec_type",
                "codec_name",
                "width",
                "height",
                "sample_rate",
                "channels",
            ]
            .iter()
            .map(|key| stream[key].to_string())
            .collect::<Vec<_>>()
            .join("|")
        })
        .collect()
}
fn chapters_match(before: &Value, after: &Value) -> bool {
    let chapters = |raw: &Value| raw["chapters"].as_array().cloned().unwrap_or_default();
    let before = chapters(before);
    let after = chapters(after);
    before.len() == after.len()
        && before.iter().zip(after.iter()).all(|(a, b)| {
            a["tags"] == b["tags"]
                && ["start_time", "end_time"].iter().all(|key| {
                    let seconds = |value: &Value| {
                        value[key]
                            .as_str()
                            .and_then(|s| s.parse::<f64>().ok())
                            .unwrap_or(0.0)
                    };
                    (seconds(a) - seconds(b)).abs() < 0.01
                })
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn artwork_rejects_invalid_data_without_creating_output() {
        let dir = tempfile::tempdir().unwrap();
        let adapter = MetadataAdapter {
            paths: RuntimePaths::new(dir.path()),
            shutdown: CancellationToken::new(),
        };
        let target = dir.path().join("cover.jpg");
        assert!(
            adapter
                .prepare_artwork("not base64!", &target)
                .await
                .is_err()
        );
        assert!(adapter.prepare_artwork("aGVsbG8=", &target).await.is_err());
        assert!(!target.exists());
    }

    #[tokio::test]
    #[ignore = "installs/checks real managed FFmpeg; run explicitly for metadata acceptance"]
    async fn real_managed_metadata_roundtrip() {
        let root = std::env::var_os("FETCH_METADATA_RUNTIME")
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::temp_dir().join("fetch-metadata-runtime"));
        let paths = RuntimePaths::new(root);
        let runtime = fetch_runtime::RuntimeManager::new(paths.clone()).unwrap();
        if !paths.ffmpeg_executable().is_file() || !paths.ffprobe_executable().is_file() {
            runtime.install_ffmpeg().await.unwrap();
        }
        let adapter = MetadataAdapter {
            paths,
            shutdown: CancellationToken::new(),
        };
        let dir = tempfile::tempdir().unwrap();
        let cover = dir.path().join("cover.jpg");
        image::RgbImage::from_pixel(32, 32, image::Rgb([200, 80, 20]))
            .save(&cover)
            .unwrap();
        for extension in ["mp3", "m4a", "flac", "mp4", "mkv"] {
            let source = dir.path().join(format!("source.{extension}"));
            let mut args: Vec<String> = [
                "-v",
                "error",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=440:duration=1",
            ]
            .into_iter()
            .map(str::to_owned)
            .collect();
            if ["mp4", "mkv"].contains(&extension) {
                args.extend(
                    [
                        "-f",
                        "lavfi",
                        "-i",
                        "color=c=blue:s=64x64:d=1",
                        "-c:v",
                        "mpeg4",
                    ]
                    .into_iter()
                    .map(str::to_owned),
                );
            }
            if extension == "mkv" {
                let subtitle = dir.path().join("subtitle.srt");
                let chapters = dir.path().join("chapters.txt");
                tokio::fs::write(
                    &subtitle,
                    "1\n00:00:00,000 --> 00:00:00,800\nPreserved subtitle\n",
                )
                .await
                .unwrap();
                tokio::fs::write(&chapters, ";FFMETADATA1\n[CHAPTER]\nTIMEBASE=1/1000\nSTART=0\nEND=1000\ntitle=Preserved chapter\n").await.unwrap();
                args.extend([
                    "-i".into(),
                    subtitle.to_string_lossy().into(),
                    "-f".into(),
                    "ffmetadata".into(),
                    "-i".into(),
                    chapters.to_string_lossy().into(),
                    "-map".into(),
                    "0:a".into(),
                    "-map".into(),
                    "1:v".into(),
                    "-map".into(),
                    "2:s".into(),
                    "-map_chapters".into(),
                    "3".into(),
                    "-metadata:s:a:0".into(),
                    "language=eng".into(),
                    "-metadata:s:s:0".into(),
                    "language=rus".into(),
                ]);
            }
            args.extend([
                "-metadata".into(),
                "title=Original".into(),
                "-metadata".into(),
                "copyright=Preserved".into(),
                source.to_string_lossy().into(),
            ]);
            adapter
                .run(
                    adapter.paths.ffmpeg_executable(),
                    &args,
                    Duration::from_secs(30),
                )
                .await
                .unwrap();
            let inspection = adapter.inspect(&source).await.unwrap();
            assert!(inspection.public.editable, "{extension}");
            let mut fields: BTreeMap<String, String> = inspection
                .public
                .supported_fields
                .iter()
                .map(|key| {
                    (
                        key.clone(),
                        match key.as_str() {
                            "track" | "disc" => "1",
                            "track_total" | "disc_total" => "2",
                            "date" => "2026",
                            _ => "Тест metadata",
                        }
                        .into(),
                    )
                })
                .collect();
            fields.remove("copyright");
            let update = MetadataUpdate {
                revision: inspection.public.revision.clone(),
                fields,
                artwork: ArtworkUpdate::Replace {
                    data: String::new(),
                },
            };
            let target = dir.path().join(format!("edited.{extension}"));
            let result = adapter
                .write(&source, &target, &inspection, &update, Some(&cover))
                .await
                .unwrap_or_else(|error| panic!("{extension}: {error:?}"));
            assert!(result.public.artwork_available, "{extension}");
            let hash_args = |path: &Path| {
                vec![
                    "-v".into(),
                    "error".into(),
                    "-i".into(),
                    path.to_string_lossy().into(),
                    "-map".into(),
                    "0:a?".into(),
                    "-map".into(),
                    "0:V?".into(),
                    "-c".into(),
                    "copy".into(),
                    "-f".into(),
                    "streamhash".into(),
                    "pipe:1".into(),
                ]
            };
            let before_hash = adapter
                .run(
                    adapter.paths.ffmpeg_executable(),
                    &hash_args(&source),
                    Duration::from_secs(30),
                )
                .await
                .unwrap();
            let after_hash = adapter
                .run(
                    adapter.paths.ffmpeg_executable(),
                    &hash_args(&target),
                    Duration::from_secs(30),
                )
                .await
                .unwrap();
            assert_eq!(
                before_hash, after_hash,
                "encoded media packets changed: {extension}"
            );
            if extension == "mkv" {
                let before_streams = inspection.raw["streams"].as_array().unwrap();
                let after_streams = result.raw["streams"].as_array().unwrap();
                assert_eq!(before_streams.len() + 1, after_streams.len());
                for (before, after) in before_streams.iter().zip(after_streams) {
                    assert_eq!(before["tags"]["language"], after["tags"]["language"]);
                }
                assert_eq!(
                    result.raw["chapters"][0]["tags"]["title"],
                    "Preserved chapter"
                );
            }

            assert!(!adapter.artwork(&target, &result).await.unwrap().is_empty());
            let kept = dir.path().join(format!("kept.{extension}"));
            let keep = MetadataUpdate {
                revision: result.public.revision.clone(),
                fields: BTreeMap::from([("title".into(), "Keep artwork".into())]),
                artwork: ArtworkUpdate::Keep,
            };
            let result = adapter
                .write(&target, &kept, &result, &keep, None)
                .await
                .unwrap_or_else(|error| panic!("keep {extension}: {error:?}"));
            assert!(result.public.artwork_available);
            let clear = MetadataUpdate {
                revision: result.public.revision.clone(),
                fields: update
                    .fields
                    .keys()
                    .map(|key| (key.clone(), String::new()))
                    .collect(),
                artwork: ArtworkUpdate::Remove,
            };
            let cleared = dir.path().join(format!("cleared.{extension}"));
            let result = adapter
                .write(&kept, &cleared, &result, &clear, None)
                .await
                .unwrap_or_else(|error| panic!("clear {extension}: {error:?}"));
            assert!(!result.public.artwork_available);
            assert_eq!(result.public.fields["copyright"], "Preserved");
        }
    }
}
