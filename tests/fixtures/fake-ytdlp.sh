#!/usr/bin/env sh
set -eu

if [ "${1:-}" = "--version" ]; then
  printf '%s\n' '2026.08.09-test'
  exit 0
fi

mode=download
for argument in "$@"; do
  if [ "$argument" = "--dump-single-json" ]; then
    mode=inspect
  fi
done

case "$mode" in
  inspect)
    for url in "$@"; do :; done
    case "${url:-}" in
      *unsupported*)
        printf '%s\n' 'ERROR: Unsupported URL' >&2
        exit 1
        ;;
      *playlist*)
        printf '%s\n' '{"_type":"playlist","id":"fixture-list","title":"Fixture playlist","entries":[{"id":"one","title":"One","url":"https://example.test/one"}]}'
        ;;
      *)
        printf '%s\n' '{"id":"fixture","title":"Fixture media","extractor_key":"Fixture","duration":12,"formats":[{"format_id":"one","ext":"mp4","vcodec":"avc1","acodec":"aac","height":720}]}'
        ;;
    esac
    ;;
  *)
    output_dir=/tmp
    thumbnail_dir=
    thumbnail_stem=
    proxy_value='<system>'
    js_runtime_value='<none>'
    previous=
    for argument in "$@"; do
      if [ "$previous" = "--paths" ]; then
        case "$argument" in
          thumbnail:*) thumbnail_dir=${argument#thumbnail:} ;;
          *) output_dir=$argument ;;
        esac
      fi
      if [ "$previous" = "--proxy" ]; then
        proxy_value=$argument
      fi
      if [ "$previous" = "--js-runtimes" ]; then
        js_runtime_value=$argument
      fi
      if [ "$previous" = "--output" ]; then
        case "$argument" in
          thumbnail:*)
            thumbnail_stem=${argument#thumbnail:}
            thumbnail_stem=${thumbnail_stem%%.*}
            ;;
        esac
      fi
      previous=$argument
    done
    case "${argument:-}" in
      *slow*) sleep 30 ;;
      *fail*) printf '%s\n' 'ERROR: fixture download failed' >&2; exit 1 ;;
    esac
    mkdir -p "$output_dir"
    case "${argument:-}" in
      *record-proxy*) printf '%s' "$proxy_value" > "$output_dir/fixture-proxy.txt" ;;
    esac
    case "${argument:-}" in
      *record-js-runtime*) printf '%s' "$js_runtime_value" > "$output_dir/fixture-js-runtime.txt" ;;
    esac
    output_file="$output_dir/fixture.mp4"
    printf '%s' 'fixture media bytes' > "$output_file"
    if [ -n "$thumbnail_dir" ] && [ -n "$thumbnail_stem" ]; then
      mkdir -p "$thumbnail_dir"
      printf '%s' 'fixture jpeg bytes' > "$thumbnail_dir/$thumbnail_stem.jpg"
    fi
    printf '%s\n' 'fetch-progress:25|100|25|3'
    printf '%s\n' 'fetch-postprocess:started'
    printf '%s\n' "fetch-file:$output_file"
    ;;
esac
