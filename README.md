# pathfinder
WebSocket-over-Kafka reverse proxy

# Dependencies
```toml
[dependencies]
clap = "2.26.2"
json = "0.11.9"
ws = "0.7.3"
yaml-rust = "0.3"

[dependencies.rdkafka]
version = "0.12.0"
features = ["ssl", "sasl"]
```

# Configuration file

# Example of configuration file
```yaml
endpoints:
  - search:
    - url: "/api/matchmaking/search"
    - microservice: "microservice.search"
  - leaderboard:
    - url: "/api/matchmaking/leaderboard"
    - microservice: "microservice.leaderboard"
websocket:
  - "127.0.0.1:9000"
kafka:
  - "127.0.0.1:9092"
```

# Documentation
Information about why this reverse proxy was implemented you can find [here](https://github.com/OpenMatchmaking/documentation/blob/master/docs/components.md#reverse-proxy).

# License
The pathfinder is published under BSD license. For more details read the [LICENSE](https://github.com/OpenMatchmaking/pathfinder/blob/master/LICENSE) file.
