# ytfeed

`ytfeed` is a feed server that provides better RSS/Atom feeds for YouTube.
It works by providing a proxy around the official feeds and obtaining additional information by scraping the site.
A simple URL interface is provided, allowing access to feeds by channel handle and optional filtering.

## Features

- Block YouTube shorts
- [DeArrow](https://dearrow.ajay.app/)'d titles
- Filter on video statistics
- Specify channels more easily using channel handles
- Hides sponsor messages from video descriptions
- Shows video statistics in your feed reader

## Installation

- With [Nix](https://nixos.org/) Flakes: `nix profile install github:rijkvp/ytfeed`. 
- Without Nix: `cargo build --release`, the resulting binary will be in `./target/release/ytfeed`.

## Usage

Feeds can be accessed by providing a channel handle in the URL path (starting with '@').

### Filter options

Filters can be applied by specifying the following query parameters:

Parameter | Description | Type
--- | --- | --- 
`d` | Mininum video duration (in seconds) | integer
`v` | Number of views | integer
`l` | Number of likes | integer
`lvr` | Like-view ratio (like / views * 100) | boolean

Note that YouTube shorts are filtered out by default, you don't have to explicitly filter for them.

## Examples

Replace `http://example.com/` with the URL of your instance.

A feed from @MyFavouriteChannel, but now better:
```
http://example.com/@MyFavouriteChannel
```

Filter on videos for channel `@ChannelHandle` longer than 10 minutes (600 seconds):
```
http://example.com/@ChannelHandle?d=600
```

Filter videos from channel `@MyChannel` with over 100,000 views and 10,000 likes:
```
http://example.com/@MyChannel?v=100000&l=10000
```

## Configuration

See using `ytfeed --help`
```
-s, --socket <SOCKET>          Socket to bind the server to [default: 0.0.0.0:8000]
-t, --timeout <CACHE_TIMEOUT>  Time to keep feeds in server cache before refreshing (in seconds) [default: 300]
```
