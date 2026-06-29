# Permissions System

Each device has a set of **granted permissions** that control what it can do. Permissions are assigned during pairing or updated later via the server admin API.

## Permission Model

- **Device-based authorization**: the `device_id` in the token determines which permissions apply
- Permissions are scoped to the pairing server's domain
- Every API endpoint checks the required permission before allowing access

## All Permissions

| Permission | Description | Typical Roles |
|---|---|---|
| `server.read` | Read server info, health, status | all devices |
| `library.read` | Browse/search the music library | controller, player, cli |
| `library.write` | Add/edit/remove library entries | cli, admin |
| `library.scan` | Trigger a library scan/re-scan | cli, admin |
| `track.read` | Read track metadata | controller, player, cli |
| `track.write` | Edit track metadata | cli, admin |
| `stream.read` | Stream audio from server | player, speaker |
| `stream.transcode` | Request on-the-fly transcoding | player, speaker |
| `artwork.read` | Fetch cover art images | controller, player, cli |
| `playlist.read` | View playlists | controller, player, cli |
| `playlist.write` | Create/edit/delete playlists | controller, cli |
| `sync.read_manifest` | Read sync manifest | sync client |
| `sync.download_tracks` | Download tracks for offline use | sync client, mobile |
| `sync.download_covers` | Download cover art for offline use | sync client, mobile |
| `sync.upload_state` | Report sync/download state | sync client |
| `playback.read` | Read current playback state | controller, cli |
| `playback.control` | Play, pause, stop, skip | controller, cli |
| `queue.read` | View playback queue | controller, player, cli |
| `queue.write` | Add/reorder/remove from queue | controller, cli |
| `receiver.read` | Read receiver info | controller, cli |
| `receiver.control` | Start/stop receiver sessions | controller |
| `receiver.session` | Manage active receiver sessions | controller |
| `receiver.volume` | Control receiver volume | controller, player |
| `room.read` | Read room configuration | controller |
| `room.write` | Edit room configuration | admin |
| `home_assistant.read` | Read Home Assistant entity state | controller |
| `home_assistant.control` | Control Home Assistant entities | controller |
| `system.read` | Read system configuration | admin, cli |
| `system.write` | Modify system configuration | admin |

## Permission Inheritance

Devices inherit permissions based on their role(s):

| Role | Inherited Permissions |
|---|---|
| `server` | All `server.*`, `system.*`, `library.*` |
| `controller` | `server.read`, `library.read`, `track.read`, `artwork.read`, `playlist.*`, `playback.*`, `queue.*`, `receiver.*`, `room.*` |
| `player` | `server.read`, `library.read`, `track.read`, `artwork.read`, `stream.*`, `queue.read`, `playback.read` |
| `sync` | `server.read`, `sync.*` |
| `mobile` | Role-based; typically `sync.*`, `playback.*`, `library.read` |
| `cli` | `server.*`, `library.*`, `system.*`, `playback.*`, `queue.*` |

## Checking Permissions

Every API endpoint declares its required permission. The server middleware:

1. Extracts the token from the `Authorization` header
2. Resolves the device and its granted permissions
3. Checks the required permission against the granted set
4. Returns **403 Forbidden** if the permission is not present

Example check in pseudo-code:
```
function authorize(device, required_permission):
    if required_permission not in device.permissions:
        return 403
    return 200
```
