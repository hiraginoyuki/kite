# Kite
> Kite is a simple reverse-proxy for Minecraft servers.

## Features
Kite can be configured to:
- restrict access to a specific domain for connecting to the server, and
- route connections to different backend servers based on the domain name used.

## Motivation
I wanted to run multiple Minecraft servers on a single machine, but I didn't want to have to open multiple ports on my router. I also wanted to be able to restrict access to each server to a specific domain name.

## Installation
```sh
cargo install --git https://github.com/hiraginoyuki/kite
```

## Usage

To use Kite, first configure it using a TOML file, and then run `kite`.

| Option | Description | Default |
| --- | --- | --- |
| -c, --config | Path to the config file. | `./kite.toml` |

To start the proxy, run the command like this:

```sh
kite -c /path/to/kite.toml
```

### Configuration
Here is an example configuration file:

```toml
# Path: kite.toml
[listen]
host = "0.0.0.0" # default, can be omitted
port = 25565 # default, can be omitted

[[rule]]
match = "pvp.example.com"
host = "127.0.0.1"
port = 25566

[[rule]]
match = "creative.example.com"
host = "160.16.63.79"
port = 25567

[[rule]]
match = "test.example.com"
host = "purpur_test" # locally resolvable hostname
port = 25565 # default, can be omitted
```

#### Note
- The `match` field can accept a domain name. However, note that the domain must resolve to your server's public address for users to be able to connect to it.
- The `host` field can accept either an IP address or a domain name. If a domain name is used, it will be resolved every time a client connects to your server.
