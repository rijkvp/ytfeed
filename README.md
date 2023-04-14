# ytfeed

YouTube Atom feeds, without shorts and filtering.

## Usage

When running locally at `0.0.0.0:8000`, feeds are accessible from the `http://0.0.0.0:80000/[channel]` endpoint.
Channels can be specified either by a channel handle (starting with an '@') or by a channel id (used in regular YouTube feeds).

### Filters

Filters can be applied to the feeds using the following query parameters:

Long | Short | Description 
--- | --- | ---
`count` | `c` | Maximum video count (number)
`duration` | `d` | Video duration (range)
`views` | `v` | Number of views (range)
`likes` | `l` | Number of likes (range)

Ranges must be specified as `a-b` where `a` is the minimum and `b` is the maximum. 
Either the minimum or maximum may be omitted (see examples below).

### Examples

Filter out videos shorter than 10 minutes seconds:
```
http:://0.0.0.0:8000/@MyChannelName?d=600-
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
  -c, --cache <CACHE_TIMEOUT>  How long to keep videos cached (in seconds) [default: 300]
  -h, --help                   Print help
  -V, --version                Print version
```
