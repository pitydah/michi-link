# Discovery Protocol

Devices on the local network discover each other using two concurrent methods: **UDP multicast announce** and **mDNS**.

## UDP Multicast Announce

Every 30 seconds, each device broadcasts a JSON payload over UDP to `255.255.255.255:42069`.

### Announce Payload

```json
{
  "device_id": "uuid-v4",
  "device_name": "Kitchen Speaker",
  "device_type": "speaker",
  "roles": ["player", "receiver"],
  "api_version": "1.0",
  "host": "192.168.1.42",
  "port": 8920,
  "capabilities": {
    "streaming": ["mp3", "flac", "aac"],
    "transcoding": true,
    "sync": true,
    "display": false
  }
}
```

| Field | Type | Description |
|-------|------|-------------|
| `device_id` | string | UUID v4, unique per device |
| `device_name` | string | Human-readable name |
| `device_type` | string | e.g. `speaker`, `server`, `cli`, `mobile` |
| `roles` | string[] | Roles this device can fulfill |
| `api_version` | string | Semver API version |
| `host` | string | IP address |
| `port` | number | HTTP API port |
| `capabilities` | object | Feature flags |

### Timeout

A device is considered offline after **3 consecutive missed announces** (90 seconds without a packet).

## mDNS

Devices also advertise via multicast DNS using the service type:

```
_michi-link._tcp.local
```

### TXT Records

Capabilities are exposed as key=value TXT records:

| Key | Example Value |
|-----|---------------|
| `device_id` | `a1b2c3d4-...` |
| `device_type` | `speaker` |
| `roles` | `player,receiver` |
| `api_version` | `1.0` |
| `streaming` | `mp3,flac,aac` |
| `transcoding` | `true` |
| `sync` | `true` |
| `display` | `false` |

## Discovery Response

When a device receives an announce packet, it should update its local device registry and may optionally respond with its own announce.

## Listening for Devices

1. Join UDP multicast group `255.255.255.255` on port `42069`
2. Listen for JSON payloads
3. Parse and register device by `device_id`
4. Reset a 90-second timeout on each received announce
5. Remove device from registry if timeout expires
