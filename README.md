# ytfeed

`ytfeed` is a feed server that provides better RSS/Atom feeds for YouTube.
It works by providing a proxy around the official feeds and obtaining additional information by scraping the site.
A simple URL interface is provided, allowing access to feeds by channel handle and optional filtering.

## Features

- Block YouTube shorts
- Filter on video statistics
- Specify channels more easily using channel handles
- Hides sponsor messages from video descriptions
- Shows video statistics in your feed reader

## Installation

- With [Nix](https://nixos.org/) Flakes: `nix profile install github:rijkvp/ytfeed`. 
- Without Nix: `cargo build --release`, the resulting binary will be in `./target/release/ytfeed`.

## Usage

Feeds can be accessed by providing a channel in the URL path. Channels can be specified either by a channel handle (starting with an '@') or by a channel ID used before handles.

### Filters

Filters can be applied by specifying the following query parameters:

Parameter | Description 
--- | ---
`c` | Maximum video count (number)
`d` | Video duration (range)
`v` | Number of views (range)
`l` | Number of likes (range)

Ranges must be specified as `a-b` where `a` is the minimum and `b` is the maximum. 
Either the minimum or maximum may be omitted (see examples below).
Note that YouTube shorts are filtered out by default, you don't have to explicitly filter for them.

## Examples

Replace `http://example.com/` by the location your instance is running.

A feed from @MyFavouriteChannel, but now better:
```
http://example.com/@MyFavouriteChannel
```

Filter on videos for channel `@ChannelHandle` longer than 10 minutes (600 seconds):
```
http://example.com/@ChannelHandle?d=600-
```

Filter videos from channel ID `UCabcdefghijklmnopqrstuv` on the last 8 videos with over 100,000 views and 10,000 likes:
```
http://example.com/UCabcdefghijklmnopqrstuv?c=8&views=100000-&likes=10000-
```

## Configuration

See using `ytfeed --help`
```
-s, --socket <SOCKET>          Socket to bind the server to [default: 0.0.0.0:8000]
-t, --timeout <CACHE_TIMEOUT>  Time to keep feeds in server cache before refreshing (in seconds) [default: 300]
```
