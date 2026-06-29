# Library API

The library is modeled around four core entities: **tracks**, **albums**, **artists**, and **playlists**.

## Track Fields

```json
{
  "id": "uuid",
  "title": "Bohemian Rhapsody",
  "artist": "Queen",
  "album": "A Night at the Opera",
  "duration": 354.5,
  "track_number": 1,
  "disc_number": 1,
  "year": 1975,
  "genre": "Rock",
  "format": "flac",
  "bitrate": 1411,
  "sample_rate": 44100,
  "channels": 2,
  "size": 62345678,
  "path": "/music/Queen/A Night at the Opera/01 - Bohemian Rhapsody.flac",
  "md5": "d41d8cd98f00b204e9800998ecf8427e",
  "added_at": "2024-01-15T10:30:00Z",
  "modified_at": "2024-01-15T10:30:00Z",
  "artwork_id": "uuid"
}
```

## Album Fields

```json
{
  "id": "uuid",
  "title": "A Night at the Opera",
  "artist": "Queen",
  "year": 1975,
  "genre": "Rock",
  "track_count": 12,
  "duration": 2580.0,
  "artwork_id": "uuid",
  "added_at": "2024-01-15T10:30:00Z"
}
```

## Artist Fields

```json
{
  "id": "uuid",
  "name": "Queen",
  "album_count": 15,
  "track_count": 180,
  "genre": "Rock",
  "artwork_id": "uuid",
  "added_at": "2024-01-15T10:30:00Z"
}
```

## Search Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `q` | string | Full-text search query |
| `artist` | string | Filter by artist name |
| `album` | string | Filter by album title |
| `genre` | string | Filter by genre |
| `year` | int | Filter by release year |
| `limit` | int | Max results (default 50, max 200) |
| `offset` | int | Pagination offset |

```
GET /library/tracks?q=bohemian&limit=20&offset=0
```

## Pagination

All list endpoints return a paginated envelope:

```json
{
  "items": [ ... ],
  "total": 150,
  "limit": 20,
  "offset": 0,
  "next": "/library/tracks?limit=20&offset=20"
}
```

## Sorting

| Endpoint | Sort Fields |
|----------|-------------|
| `/library/tracks` | `title`, `artist`, `album`, `year`, `added_at`, `duration` |
| `/library/albums` | `title`, `artist`, `year`, `added_at` |
| `/library/artists` | `name`, `album_count`, `added_at` |

Default sort is `title` ascending. Use `?sort=year&order=desc`.

## Library Stats

```
GET /library/stats

Response:
{
  "track_count": 25000,
  "album_count": 2400,
  "artist_count": 850,
  "playlist_count": 12,
  "total_duration": 7200000,
  "total_size": 450000000000,
  "last_scan": "2024-01-15T10:30:00Z"
}
```

## Scan Trigger

```
POST /library/scan

Response (202):
{
  "status": "scanning",
  "started_at": "2024-01-15T10:30:00Z"
}
```

Requires `library.scan` permission. The scan runs asynchronously. Poll `/library/stats` to check when `last_scan` updates.
