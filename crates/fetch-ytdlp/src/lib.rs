//! Safe command construction, media normalization, and progress parsing for yt-dlp.

use std::{path::PathBuf, process::Stdio};

use fetch_core::{
    DownloadMode, DownloadProgress, DownloadRequest, FetchError, MediaAnalysis, MediaFormat,
    MediaInfo, MediaKind, PlaylistEntry, ProxyMode, ProxySettings,
};
use serde_json::Value;
use tokio::{io::AsyncReadExt, process::Command};

const PROGRESS_PREFIX: &str = "fetch-progress:";
const POSTPROCESS_PREFIX: &str = "fetch-postprocess:";
const FILE_PREFIX: &str = "fetch-file:";

#[derive(Debug, Clone)]
pub struct DownloadArtifacts {
    pub thumbnail_directory: PathBuf,
    pub thumbnail_stem: String,
}

#[derive(Debug, Clone)]
pub struct YtDlp {
    executable: PathBuf,
    ffmpeg_directory: Option<PathBuf>,
    proxy: ProxySettings,
}

#[async_trait::async_trait]
impl MediaAnalysis for YtDlp {
    async fn analyze(&self, url: &str) -> Result<MediaInfo, FetchError> {
        self.inspect_url(url).await
    }
}

impl YtDlp {
    pub fn new(executable: impl Into<PathBuf>) -> Self {
        Self {
            executable: executable.into(),
            ffmpeg_directory: None,
            proxy: ProxySettings::default(),
        }
    }

    pub fn with_ffmpeg_directory(mut self, directory: impl Into<PathBuf>) -> Self {
        self.ffmpeg_directory = Some(directory.into());
        self
    }

    pub fn with_proxy(mut self, proxy: ProxySettings) -> Result<Self, FetchError> {
        proxy.validate()?;
        self.proxy = proxy;
        Ok(self)
    }

    pub fn executable(&self) -> &std::path::Path {
        &self.executable
    }

    pub async fn version(&self) -> Result<String, FetchError> {
        let output = Command::new(&self.executable)
            .arg("--version")
            .stdin(Stdio::null())
            .output()
            .await
            .map_err(|error| FetchError::RuntimeMissing(format!("yt-dlp: {error}")))?;
        if !output.status.success() {
            return Err(FetchError::RuntimeCorrupt("yt-dlp".into()));
        }
        let version = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        if version.is_empty() {
            return Err(FetchError::RuntimeCorrupt("yt-dlp".into()));
        }
        Ok(version)
    }

    pub async fn inspect_url(&self, url: &str) -> Result<MediaInfo, FetchError> {
        if url.trim().is_empty() {
            return Err(FetchError::InvalidRequest("URL must not be empty".into()));
        }
        let output = Command::new(&self.executable)
            .args(build_inspect_args(url, &self.proxy)?)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|error| FetchError::RuntimeMissing(format!("yt-dlp: {error}")))?;

        if !output.status.success() {
            let diagnostics = self.proxy.redact(&String::from_utf8_lossy(&output.stderr));
            return Err(map_process_error(&diagnostics));
        }
        normalize_media_json(&output.stdout)
    }

    pub fn download_command(&self, request: &DownloadRequest) -> Result<Command, FetchError> {
        self.download_command_with_artifacts(request, None)
    }

    pub fn download_command_with_artifacts(
        &self,
        request: &DownloadRequest,
        artifacts: Option<&DownloadArtifacts>,
    ) -> Result<Command, FetchError> {
        let mut command = Command::new(&self.executable);
        for argument in build_download_args_with_proxy(
            request,
            self.ffmpeg_directory.as_deref(),
            artifacts,
            &self.proxy,
        )? {
            command.arg(argument);
        }
        command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        #[cfg(unix)]
        command.process_group(0);
        #[cfg(windows)]
        command.creation_flags(0x0000_0200); // CREATE_NEW_PROCESS_GROUP
        Ok(command)
    }
}

pub fn build_inspect_args(url: &str, proxy: &ProxySettings) -> Result<Vec<String>, FetchError> {
    if url.trim().is_empty() {
        return Err(FetchError::InvalidRequest("URL must not be empty".into()));
    }
    proxy.validate()?;
    let mut args = proxy_args(proxy);
    args.extend([
        "--dump-single-json".into(),
        "--skip-download".into(),
        "--no-warnings".into(),
        "--".into(),
        url.into(),
    ]);
    Ok(args)
}

fn proxy_args(proxy: &ProxySettings) -> Vec<String> {
    match proxy.mode {
        ProxyMode::System => vec![],
        ProxyMode::Direct => vec!["--proxy".into(), String::new()],
        ProxyMode::Custom => vec![
            "--proxy".into(),
            proxy.url.clone().expect("validated custom proxy has a URL"),
        ],
    }
}

pub fn normalize_media_json(bytes: &[u8]) -> Result<MediaInfo, FetchError> {
    let value: Value =
        serde_json::from_slice(bytes).map_err(|error| FetchError::ProcessFailed {
            summary: "yt-dlp returned invalid media information".into(),
            details: error.to_string(),
        })?;
    let kind = if value.get("_type").and_then(Value::as_str) == Some("playlist") {
        MediaKind::Playlist
    } else {
        MediaKind::Media
    };
    let entries = value
        .get("entries")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            Some(PlaylistEntry {
                id: text(entry, "id"),
                title: text(entry, "title").or_else(|| text(entry, "id"))?,
                url: text(entry, "webpage_url").or_else(|| text(entry, "url")),
                duration_seconds: number(entry, "duration"),
                thumbnail_url: text(entry, "thumbnail"),
            })
        })
        .collect::<Vec<_>>();
    let formats = value
        .get("formats")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(normalize_format)
        .collect::<Vec<_>>();
    let title = text(&value, "title")
        .or_else(|| text(&value, "id"))
        .ok_or_else(|| FetchError::ProcessFailed {
            summary: "yt-dlp response did not include a media title".into(),
            details: String::new(),
        })?;

    Ok(MediaInfo {
        kind,
        id: text(&value, "id"),
        extractor: text(&value, "extractor_key").or_else(|| text(&value, "extractor")),
        title,
        webpage_url: text(&value, "webpage_url"),
        duration_seconds: number(&value, "duration"),
        thumbnail_url: text(&value, "thumbnail"),
        playlist_count: value
            .get("playlist_count")
            .and_then(Value::as_u64)
            .or_else(|| (kind == MediaKind::Playlist).then_some(entries.len() as u64)),
        entries,
        formats,
    })
}

fn normalize_format(value: &Value) -> Option<MediaFormat> {
    let id = text(value, "format_id")?;
    let video_codec = codec(value, "vcodec");
    let audio_codec = codec(value, "acodec");
    let width = integer(value, "width");
    let height = integer(value, "height");
    let extension = text(value, "ext");
    let label = text(value, "format_note")
        .or_else(|| text(value, "format"))
        .unwrap_or_else(|| {
            let resolution = height.map(|height| format!("{height}p"));
            [resolution, extension.clone()]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join(" · ")
        });
    Some(MediaFormat {
        id,
        label,
        extension,
        has_video: video_codec.is_some(),
        has_audio: audio_codec.is_some(),
        video_codec,
        audio_codec,
        width,
        height,
        fps: number(value, "fps"),
        bitrate_kbps: number(value, "tbr").or_else(|| number(value, "abr")),
        filesize_bytes: value
            .get("filesize")
            .and_then(Value::as_u64)
            .or_else(|| value.get("filesize_approx").and_then(Value::as_u64)),
    })
}

fn text(value: &Value, field: &str) -> Option<String> {
    value.get(field)?.as_str().map(ToOwned::to_owned)
}

fn number(value: &Value, field: &str) -> Option<f64> {
    value.get(field)?.as_f64()
}

fn integer(value: &Value, field: &str) -> Option<u32> {
    value.get(field)?.as_u64()?.try_into().ok()
}

fn codec(value: &Value, field: &str) -> Option<String> {
    text(value, field).filter(|codec| codec != "none")
}

pub fn build_download_args(
    request: &DownloadRequest,
    ffmpeg_directory: Option<&std::path::Path>,
    artifacts: Option<&DownloadArtifacts>,
) -> Result<Vec<String>, FetchError> {
    build_download_args_with_proxy(
        request,
        ffmpeg_directory,
        artifacts,
        &ProxySettings::default(),
    )
}

pub fn build_download_args_with_proxy(
    request: &DownloadRequest,
    ffmpeg_directory: Option<&std::path::Path>,
    artifacts: Option<&DownloadArtifacts>,
    proxy: &ProxySettings,
) -> Result<Vec<String>, FetchError> {
    if request.url.trim().is_empty() {
        return Err(FetchError::InvalidRequest("URL must not be empty".into()));
    }
    let output_directory = request.output_directory.as_ref().ok_or_else(|| {
        FetchError::OutputDirectoryUnavailable("no output directory was configured".into())
    })?;
    proxy.validate()?;
    let mut args = proxy_args(proxy);
    args.extend([
        "--newline".into(),
        "--continue".into(),
        "--windows-filenames".into(),
        "--trim-filenames".into(),
        "180".into(),
        "--progress-template".into(),
        "download:fetch-progress:%(progress.downloaded_bytes)s|%(progress.total_bytes,progress.total_bytes_estimate)s|%(progress.speed)s|%(progress.eta)s".into(),
        "--progress-template".into(),
        "postprocess:fetch-postprocess:%(progress.status)s".into(),
        "--print".into(),
        "after_move:fetch-file:%(filepath)s".into(),
    ]);
    if let Some(artifacts) = artifacts {
        args.extend([
            "--write-thumbnail".into(),
            "--convert-thumbnails".into(),
            "jpg".into(),
            "--paths".into(),
            format!(
                "thumbnail:{}",
                artifacts.thumbnail_directory.to_string_lossy()
            ),
            "--output".into(),
            format!("thumbnail:{}.%(ext)s", artifacts.thumbnail_stem),
        ]);
    }
    let output_template = request.playlist.as_ref().map_or_else(
        || "%(title)s [%(id)s].%(ext)s".to_owned(),
        |playlist| format!("{:03} - %(title)s [%(id)s].%(ext)s", playlist.index),
    );
    args.extend([
        "--paths".into(),
        output_directory.to_string_lossy().into_owned(),
        "--output".into(),
        output_template,
    ]);
    if let Some(directory) = ffmpeg_directory {
        args.extend([
            "--ffmpeg-location".into(),
            directory.to_string_lossy().into_owned(),
        ]);
    }
    match request.mode {
        DownloadMode::Video => {
            let format = request
                .format_id
                .clone()
                .unwrap_or_else(|| video_format_selector(request));
            args.extend(["--format".into(), format]);
            if let Some(container) = normalized_choice(request.container.as_deref()) {
                args.extend(["--merge-output-format".into(), container]);
            }
        }
        DownloadMode::Audio => {
            args.extend([
                "--format".into(),
                request
                    .format_id
                    .clone()
                    .unwrap_or_else(|| "bestaudio/best".into()),
                "--extract-audio".into(),
            ]);
            if let Some(codec) = normalized_choice(request.audio_codec.as_deref()) {
                args.extend(["--audio-format".into(), codec]);
            }
        }
    }
    if request.embed_metadata {
        args.push("--embed-metadata".into());
    }
    if request.embed_thumbnail {
        args.push("--embed-thumbnail".into());
    }
    if request.subtitles {
        args.extend(["--write-subs".into(), "--embed-subs".into()]);
    }
    args.extend(["--".into(), request.url.clone()]);
    Ok(args)
}

fn normalized_choice(value: Option<&str>) -> Option<String> {
    value
        .filter(|value| !value.is_empty() && !value.eq_ignore_ascii_case("auto"))
        .map(|value| value.to_ascii_lowercase())
}

fn video_format_selector(request: &DownloadRequest) -> String {
    let height = request.quality.as_deref().and_then(|quality| {
        quality
            .trim_end_matches(['p', 'P'])
            .parse::<u32>()
            .ok()
            .filter(|height| (144..=4320).contains(height))
    });
    let codec_prefix = match request.video_codec.as_deref() {
        Some(codec) if codec.eq_ignore_ascii_case("h264") => Some("avc"),
        Some(codec) if codec.eq_ignore_ascii_case("av1") => Some("av01"),
        Some(codec) if codec.eq_ignore_ascii_case("vp9") => Some("vp9"),
        _ => None,
    };
    let height_filter = height
        .map(|height| format!("[height<={height}]"))
        .unwrap_or_default();
    let codec_filter = codec_prefix
        .map(|codec| format!("[vcodec^={codec}]"))
        .unwrap_or_default();
    format!("bestvideo{height_filter}{codec_filter}+bestaudio/best{height_filter}")
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProcessLine {
    Progress(DownloadProgress),
    Postprocessing,
    CompletedFile(PathBuf),
    Diagnostic(String),
}

pub fn parse_process_line(line: &str) -> ProcessLine {
    if let Some(payload) = line.strip_prefix(PROGRESS_PREFIX) {
        let fields = payload.split('|').collect::<Vec<_>>();
        let downloaded_bytes = parse_u64(fields.first().copied());
        let total_bytes = parse_u64(fields.get(1).copied());
        return ProcessLine::Progress(DownloadProgress {
            progress_percent: downloaded_bytes
                .zip(total_bytes)
                .filter(|(_, total)| *total > 0)
                .map(|(downloaded, total)| downloaded as f64 / total as f64 * 100.0),
            downloaded_bytes,
            total_bytes,
            speed_bytes_per_second: parse_u64(fields.get(2).copied()),
            eta_seconds: parse_u64(fields.get(3).copied()),
        });
    }
    if line.starts_with(POSTPROCESS_PREFIX) {
        return ProcessLine::Postprocessing;
    }
    if let Some(path) = line.strip_prefix(FILE_PREFIX) {
        return ProcessLine::CompletedFile(PathBuf::from(path));
    }
    ProcessLine::Diagnostic(line.to_owned())
}

fn parse_u64(value: Option<&str>) -> Option<u64> {
    value?.trim().parse().ok()
}

pub fn map_process_error(stderr: &str) -> FetchError {
    let lower = stderr.to_ascii_lowercase();
    if lower.contains("unsupported url")
        || lower.contains("no suitable extractor")
        || lower.contains("not a valid url")
    {
        FetchError::UnsupportedUrl
    } else if lower.contains("sign in")
        || lower.contains("login required")
        || lower.contains("authentication")
    {
        FetchError::AuthenticationRequired
    } else if lower.contains("not available in your country") || lower.contains("geo restricted") {
        FetchError::GeoRestricted
    } else if lower.contains("video unavailable") || lower.contains("media unavailable") {
        FetchError::MediaUnavailable
    } else {
        let summary = stderr
            .lines()
            .rev()
            .find(|line| !line.trim().is_empty())
            .unwrap_or("yt-dlp failed")
            .trim()
            .trim_start_matches("ERROR:")
            .trim()
            .to_owned();
        FetchError::ProcessFailed {
            summary,
            details: stderr.trim().to_owned(),
        }
    }
}

pub async fn read_to_string(
    mut stream: impl tokio::io::AsyncRead + Unpin,
) -> std::io::Result<String> {
    let mut output = String::new();
    stream.read_to_string(&mut output).await?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_single_media_and_formats() {
        let media = normalize_media_json(
            br#"{"id":"abc","title":"Example","extractor_key":"Generic","duration":42.5,"formats":[{"format_id":"v1","format_note":"1080p","ext":"mp4","vcodec":"avc1","acodec":"none","height":1080,"filesize":1000}]}"#,
        )
        .unwrap();
        assert_eq!(media.kind, MediaKind::Media);
        assert_eq!(media.extractor.as_deref(), Some("Generic"));
        assert_eq!(media.formats[0].height, Some(1080));
        assert!(media.formats[0].has_video);
        assert!(!media.formats[0].has_audio);
    }

    #[test]
    fn distinguishes_playlist() {
        let media = normalize_media_json(
            br#"{"_type":"playlist","id":"list","title":"A list","entries":[{"id":"one","title":"One","url":"https://example.test/1","thumbnail":"https://example.test/one.jpg"}]}"#,
        )
        .unwrap();
        assert_eq!(media.kind, MediaKind::Playlist);
        assert_eq!(media.id.as_deref(), Some("list"));
        assert_eq!(media.playlist_count, Some(1));
        assert_eq!(
            media.entries[0].thumbnail_url.as_deref(),
            Some("https://example.test/one.jpg")
        );
    }

    #[test]
    fn maps_errors_without_site_specific_branches() {
        assert!(matches!(
            map_process_error("ERROR: Unsupported URL"),
            FetchError::UnsupportedUrl
        ));
        assert!(matches!(
            map_process_error("login required"),
            FetchError::AuthenticationRequired
        ));
    }

    #[test]
    fn parses_machine_progress() {
        let line = parse_process_line("fetch-progress:50|100|25|2");
        assert_eq!(
            line,
            ProcessLine::Progress(DownloadProgress {
                progress_percent: Some(50.0),
                downloaded_bytes: Some(50),
                total_bytes: Some(100),
                speed_bytes_per_second: Some(25),
                eta_seconds: Some(2)
            })
        );
    }

    #[test]
    fn arguments_are_deterministic_and_keep_url_separate() {
        let request = DownloadRequest {
            url: "https://example.test/a?x=$(bad)".into(),
            title: None,
            duration_seconds: None,
            mode: DownloadMode::Audio,
            format_id: None,
            quality: None,
            container: None,
            video_codec: None,
            audio_codec: Some("mp3".into()),
            embed_metadata: true,
            embed_thumbnail: false,
            subtitles: false,
            playlist: None,
            output_directory: Some(PathBuf::from("/tmp/out")),
        };
        let args =
            build_download_args(&request, Some(std::path::Path::new("/runtime")), None).unwrap();
        assert_eq!(args.last(), Some(&request.url));
        assert!(args.iter().any(|argument| argument == "--continue"));
        assert!(
            args.windows(2)
                .any(|pair| pair == ["--audio-format", "mp3"])
        );
        assert!(
            args.windows(2)
                .any(|pair| pair == ["--ffmpeg-location", "/runtime"])
        );
    }

    #[test]
    fn proxy_modes_apply_to_analysis_and_download_commands() {
        let request = DownloadRequest {
            url: "https://example.test/media".into(),
            title: None,
            duration_seconds: None,
            mode: DownloadMode::Video,
            format_id: None,
            quality: None,
            container: None,
            video_codec: None,
            audio_codec: None,
            embed_metadata: false,
            embed_thumbnail: false,
            subtitles: false,
            playlist: None,
            output_directory: Some(PathBuf::from("/tmp/out")),
        };
        let cases = [
            (ProxySettings::default(), None),
            (
                ProxySettings {
                    mode: ProxyMode::Direct,
                    url: None,
                },
                Some(""),
            ),
            (
                ProxySettings {
                    mode: ProxyMode::Custom,
                    url: Some("socks5://127.0.0.1:1080".into()),
                },
                Some("socks5://127.0.0.1:1080"),
            ),
        ];
        for (proxy, expected) in cases {
            let inspect = build_inspect_args(&request.url, &proxy).unwrap();
            let download = build_download_args_with_proxy(&request, None, None, &proxy).unwrap();
            for args in [&inspect, &download] {
                let actual = args
                    .windows(2)
                    .find(|pair| pair[0] == "--proxy")
                    .map(|pair| pair[1].as_str());
                assert_eq!(actual, expected);
                assert_eq!(args.last(), Some(&request.url));
            }
        }
    }

    #[test]
    fn video_preferences_build_a_bounded_selector() {
        let request = DownloadRequest {
            url: "https://example.test/video".into(),
            title: None,
            duration_seconds: None,
            mode: DownloadMode::Video,
            format_id: None,
            quality: Some("1080p".into()),
            container: Some("mp4".into()),
            video_codec: Some("h264".into()),
            audio_codec: None,
            embed_metadata: false,
            embed_thumbnail: false,
            subtitles: false,
            playlist: None,
            output_directory: Some(PathBuf::from("/tmp/out")),
        };
        let args = build_download_args(&request, None, None).unwrap();
        assert!(args.windows(2).any(|pair| {
            pair == [
                "--format",
                "bestvideo[height<=1080][vcodec^=avc]+bestaudio/best[height<=1080]",
            ]
        }));
        assert!(
            args.windows(2)
                .any(|pair| pair == ["--merge-output-format", "mp4"])
        );
    }

    #[test]
    fn playlist_items_use_ordered_names_and_cached_thumbnail_output() {
        let request = DownloadRequest {
            url: "https://example.test/item".into(),
            title: Some("Item".into()),
            duration_seconds: Some(42.0),
            mode: DownloadMode::Video,
            format_id: None,
            quality: None,
            container: None,
            video_codec: None,
            audio_codec: None,
            embed_metadata: false,
            embed_thumbnail: false,
            subtitles: false,
            playlist: Some(fetch_core::PlaylistContext {
                id: "list".into(),
                title: "A list".into(),
                index: 7,
            }),
            output_directory: Some(PathBuf::from("/downloads/Playlists/A list [list]")),
        };
        let artifacts = DownloadArtifacts {
            thumbnail_directory: PathBuf::from("/data/thumbnails"),
            thumbnail_stem: "job-id".into(),
        };
        let args = build_download_args(&request, None, Some(&artifacts)).unwrap();
        assert!(
            args.windows(2)
                .any(|pair| pair == ["--output", "007 - %(title)s [%(id)s].%(ext)s"])
        );
        assert!(
            args.windows(2)
                .any(|pair| pair == ["--paths", "thumbnail:/data/thumbnails"])
        );
        assert!(
            args.windows(2)
                .any(|pair| pair == ["--output", "thumbnail:job-id.%(ext)s"])
        );
        assert!(args.iter().any(|argument| argument == "--write-thumbnail"));
        assert!(
            args.iter()
                .any(|argument| argument == "--convert-thumbnails")
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn fixture_exercises_real_command_invocation() {
        use std::os::unix::fs::PermissionsExt;

        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/fake-ytdlp.sh");
        let mut permissions = std::fs::metadata(&fixture).unwrap().permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&fixture, permissions).unwrap();
        let adapter = YtDlp::new(fixture);
        assert_eq!(adapter.version().await.unwrap(), "2026.08.09-test");
        let media = adapter
            .inspect_url("https://example.test/media")
            .await
            .unwrap();
        assert_eq!(media.title, "Fixture media");
        assert!(matches!(
            adapter
                .inspect_url("https://example.test/unsupported")
                .await,
            Err(FetchError::UnsupportedUrl)
        ));
    }
}
