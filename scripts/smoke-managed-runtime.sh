#!/usr/bin/env bash
set -euo pipefail

binary="${1:?usage: smoke-managed-runtime.sh <fetch-binary> [media-url]}"
media_url="${2:-https://media.w3.org/2010/05/sintel/trailer.mp4}"
command -v jq >/dev/null 2>&1 || { printf 'jq is required\n' >&2; exit 2; }
smoke_root="$(mktemp -d)"
cleanup() {
  if [[ -n "${fetch_pid:-}" ]]; then kill "$fetch_pid" 2>/dev/null || true; fi
  rm -rf "$smoke_root"
}
trap cleanup EXIT
mkdir -p "$smoke_root/data" "$smoke_root/downloads"
config="$smoke_root/fetch.toml"
printf 'bind_address = "127.0.0.1"\nport = 18992\ndata_directory = "%s"\ndownload_directory = "%s"\nopen_browser_on_start = false\nytdlp_auto_update = false\n' "$smoke_root/data" "$smoke_root/downloads" > "$config"
FETCH_CONFIG="$config" "$binary" >"$smoke_root/fetch.log" 2>&1 &
fetch_pid=$!
for _ in $(seq 1 200); do
  curl --fail --silent http://127.0.0.1:18992/api/status >/dev/null 2>&1 && break
  sleep 0.1
done

curl --fail --silent -X POST http://127.0.0.1:18992/api/runtime/yt-dlp/update >/dev/null
curl --fail --silent -X POST http://127.0.0.1:18992/api/runtime/ffmpeg/update >/dev/null
for _ in $(seq 1 600); do
  runtime="$(curl --fail --silent http://127.0.0.1:18992/api/runtime)"
  if [[ "$(jq '[.[] | select(.status == "ready")] | length' <<<"$runtime")" = "3" ]] \
    && [[ -x "$smoke_root/data/runtime/yt-dlp/yt-dlp" ]] \
    && [[ -x "$smoke_root/data/runtime/ffmpeg/ffmpeg" ]]; then
    break
  fi
  sleep 1
done
[[ -x "$smoke_root/data/runtime/ffmpeg/ffprobe" ]]
curl --fail --silent -H 'Content-Type: application/json' \
  -d "$(jq -cn --arg url "$media_url" '{url:$url}')" \
  http://127.0.0.1:18992/api/media/analyze | jq -e '.title | length > 0' >/dev/null
job_id="$(curl --fail --silent -H 'Content-Type: application/json' \
  -d "$(jq -cn --arg url "$media_url" '{url:$url,title:"Managed runtime smoke",mode:"audio",format_id:null,quality:null,container:null,video_codec:null,audio_codec:"mp3",embed_metadata:false,embed_thumbnail:false,subtitles:false,output_directory:null}')" \
  http://127.0.0.1:18992/api/downloads | jq -r .id)"
for _ in $(seq 1 300); do
  job="$(curl --fail --silent "http://127.0.0.1:18992/api/downloads/$job_id")"
  status="$(jq -r .status <<<"$job")"
  [[ "$status" = "completed" ]] && break
  if [[ "$status" = "failed" ]]; then jq . <<<"$job" >&2; exit 1; fi
  sleep 1
done
[[ "${status:-}" = "completed" ]]
curl --fail --silent http://127.0.0.1:18992/api/completed | jq -e 'length == 1 and .[0].size_bytes > 0' >/dev/null
