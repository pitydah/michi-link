# Streaming Protocol

Audio is served over HTTP with support for **Range requests**, **transcoding**, and **content negotiation**.

## HTTP Range Requests

Clients request byte ranges for seeking. The server responds with `206 Partial Content`.

```
GET /stream/{track_id}?format=flac
Range: bytes=0-1048575

Response:
206 Partial Content
Content-Type: audio/flac
Content-Range: bytes 0-1048575/62345678
Accept-Ranges: bytes
```

Supported content types:
- `audio/mpeg` (.mp3)
- `audio/flac` (.flac)
- `audio/ogg` (.ogg, .opus)
- `audio/aac` (.aac)
- `audio/wav` (.wav)
- `audio/aiff` (.aiff)

## Transcoding

The server can transcode on the fly. Request a different format than the source file:

```
GET /stream/{track_id}?format=mp3&bitrate=320
```

| Parameter | Type | Description |
|-----------|------|-------------|
| `format` | string | Target format (`mp3`, `aac`, `ogg`, `flac`) |
| `bitrate` | int | Target bitrate in kbps (e.g. `128`, `192`, `320`) |
| `sample_rate` | int | Target sample rate (e.g. `44100`, `48000`) |

Requires `stream.transcode` permission. If not granted, the server returns the original file.

## Download

For offline storage:

```
GET /download/{track_id}?format=flac

Response:
200 OK
Content-Type: audio/flac
Content-Disposition: attachment; filename="Bohemian_Rhapsody.flac"
```

## Artwork

```
GET /artwork/{artwork_id}
GET /artwork/{artwork_id}?size=300

Response:
200 OK
Content-Type: image/jpeg
Cache-Control: public, max-age=86400
```

Supported sizes via `?size=` (server picks closest match): `50`, `150`, `300`, `500`, `1000`.

## Cache Headers

- Streamable audio: `Cache-Control: public, max-age=3600`
- Downloads: `Cache-Control: public, max-age=604800`
- Artwork: `Cache-Control: public, max-age=86400`
- Transcoding responses: `Cache-Control: no-cache`

## Example curl commands

```bash
# Stream a FLAC file
curl -H "Authorization: Bearer <token>" \
  "http://server:8920/stream/uuid?format=flac" \
  --output song.flac

# Stream with range (first 1MB)
curl -H "Authorization: Bearer <token>" \
  -H "Range: bytes=0-1048575" \
  "http://server:8920/stream/uuid?format=flac" \
  --output song_part.flac

# Transcode to 320kbps MP3
curl -H "Authorization: Bearer <token>" \
  "http://server:8920/stream/uuid?format=mp3&bitrate=320" \
  --output song.mp3

# Download for offline
curl -H "Authorization: Bearer <token>" \
  "http://server:8920/download/uuid?format=flac" \
  --output song.flac

# Fetch artwork
curl -H "Authorization: Bearer <token>" \
  "http://server:8920/artwork/uuid?size=300" \
  --output cover.jpg
```

## Bitrate and Format Negotiation

Clients indicate preferences via query parameters. The server selects the best match:

1. If the source matches the requested format, serve directly
2. If transcoding is allowed and the format differs, transcode
3. If transcoding is not allowed, serve the original regardless of format
4. Bitrate is clamped to the source bitrate (cannot upscale)
