# Feeds to epub

This is a small little daemon that aims to sit on your server or system to periodically wake up and read your RSS/Atom feeds to then transform each post into it's own epub file that can be consumed from wherever you want.
Currently it's still manually invoked while I iron out the kinks.

## Configuraion file format

```toml
[feeds]
  [feeds.test1]
  url = "https://test1/atom.xml"
```

## TODO

* Handle ETAG values as well
* Consider whether storing the last polled timestamp is a better solution than just polling at most every 3600, this is guaranteed because the sleep for this code is at the end and is minimum 3600s so we can never go under it unless there's something wrong with the thread sleep stuff in Rust itself.

