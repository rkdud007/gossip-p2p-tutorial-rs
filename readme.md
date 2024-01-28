# Goofy Gossip protocol p2p network

Simple gossip protcol implementation with rust libp2p library inspired by ping-pong tutorial from : https://github.com/libp2p/rust-libp2p/blob/master/libp2p/src/tutorials/ping.rs

### Run

First Terminal

```sh
cargo run
```

you will get like this

```sh
Listening on "/ip4/127.0.0.1/tcp/51323"
Listening on "/ip4/192.168.0.3/tcp/51323"
Listening on "/ip4/169.254.58.22/tcp/51323"
```

Second terminal

```sh
cargo run -- /ip4/127.0.0.1/tcp/51323
```
