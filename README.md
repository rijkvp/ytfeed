# ytfeed

YouTube Atom feeds, with filtering options.

## Usage

When running locally at `0.0.0.0:8000`, feeds are accessible from the `http://0.0.0.0:80000/[channel]` endpoint.
Channels are specified either by a channel ID used in regular YouTube feeds or by a channel name starting with an @ in the new YouTube channel URLs.

### Filters

Filters can be applied to the feeds using the following query parameters.

Long | Short | Description 
--- | --- | ---
`count` | `c` | Maximum count of items (after the filter has been applied).
`duration` | `d` | Video duration range 
`likes` | `l` | Number of likes range
`views` | `v` | Video view counter range
`live` | `e` | Whether or not the video is a livestream
`tag` | `t` | Include only videos with a specific hashtag (note that YouTube limits videos to 3 hashtags)

Ranges must be specified as `a-b` where `a` is the minimum and `b` is the maximum. The minimum or maximum may be omitted (see examples below).

### Examples

Filter out shorts, since the max duration of a YouTube short is 60 seconds:
```
http:://0.0.0.0:8000/@MyChannelName?d=61-
```
Filter the 8 videos with over 100,000 views and 10,00 likes:
```
http:://0.0.0.0:8000/UCabcdefghijklmnopqrstuv?c=8&views=100000-&likes=10000
```

## Running

The server can be configured using the command line parameters:
```
YouTube feed proxy

Usage: ytfeed [OPTIONS]

Options:
  -b, --bind <BIND_ADDRESS>    IP bind address [default: 0.0.0.0:8000]
  -l, --limit <VIDEO_LIMIT>    Maximum amount of videos to fetch per channel [default: 20]
  -c, --cache <CACHE_TIMEOUT>  How long to keep videos cached (in seconds) [default: 300]
  -h, --help                   Print help
  -V, --version                Print version
```
