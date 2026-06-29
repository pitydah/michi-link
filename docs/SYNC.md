# Sync Protocol

The sync protocol enables devices to maintain an offline copy of the music library. It uses a **two-way model** where the server tracks what each client has and the client decides what to download.

## Sync Flow

```
Client                         Server
  |                              |
  |-- GET /sync/manifest ------->|
  |<-- full manifest JSON -------|
  |                              |
  |  (diff locally: compare     |
  |   hashes against stored)    |
  |                              |
  |-- POST /sync/delta --------->|
  |   { last_sync_id,           |
  |     local_tracks[] }        |
  |<-- delta JSON ---------------|
  |                              |
  |  (download needed tracks    |
  |   via /download endpoint)   |
  |                              |
  |-- POST /sync/state --------->|
  |   { synced_tracks[],        |
  |     downloaded_tracks[],    |
  |     failed_tracks[] }       |
  |<-- 200 OK -------------------|
```

## Sync Manifest

A full manifest contains every track the server knows about:

```json
{
  "sync_id": "uuid-v4",
  "generated_at": "2024-01-15T10:30:00Z",
  "library_version": 42,
  "tracks": [
    {
      "id": "uuid",
      "title": "Bohemian Rhapsody",
      "artist": "Queen",
      "album": "A Night at the Opera",
      "duration": 354.5,
      "track_number": 1,
      "disc_number": 1,
      "year": 1975,
      "format": "flac",
      "size": 62345678,
      "md5": "d41d8cd98f00b204e9800998ecf8427e",
      "artwork_md5": "abc123def456",
      "modified_at": "2024-01-15T10:30:00Z"
    }
  ],
  "artworks": [
    {
      "id": "uuid",
      "md5": "abc123def456"
    }
  ]
}
```

## Delta Manifest

Only changes since the last sync:

```json
{
  "sync_id": "uuid-v4",
  "library_version": 43,
  "added": [
    {
      "id": "uuid",
      "title": "New Song",
      ...
    }
  ],
  "modified": [
    {
      "id": "uuid",
      "md5": "newhash123",
      ...
    }
  ],
  "removed": [
    "uuid-of-deleted-track"
  ],
  "artworks_changed": [
    {
      "id": "uuid",
      "md5": "newartworkhash"
    }
  ]
}
```

## Conflict Resolution

| Conflict | Resolution |
|----------|------------|
| Metadata changed on server | **Server wins** — client updates local metadata |
| Track deleted on server | **Server wins** — client may delete local copy or keep it |
| Client has a track server doesn't know about | Client keeps it, not reported in manifest |
| Download fails | Client retries on next sync cycle |

The client always decides **what** and **when** to download. The server only reports the authoritative state.

## Sync State Reporting

After downloading, the client reports its state:

```
POST /sync/state
Content-Type: application/json

{
  "sync_id": "uuid",
  "library_version": 43,
  "synced_tracks": ["uuid1", "uuid2", ...],
  "downloaded_tracks": ["uuid1", "uuid2", ...],
  "failed_tracks": ["uuid3"],
  "downloaded_artworks": ["art-uuid1"],
  "last_sync_at": "2024-01-15T11:00:00Z"
}
```

The server stores this state so the next delta request only includes new/updated tracks.
