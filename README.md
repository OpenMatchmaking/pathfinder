# pathfinder
WebSocket-over-Kafka reverse proxy

# Dependencies
```toml
[dependencies]
json = "0.11.9"

[dependencies.rdkafka]
version = "0.12.0"
features = ["ssl", "sasl"]
```

# Documentation
Information about why this reverse proxy was implemented you can find [here](https://github.com/OpenMatchmaking/documentation/blob/master/docs/components.md#reverse-proxy).

# License
The pathfinder is published under BSD license. For more details read the [LICENSE](https://github.com/OpenMatchmaking/pathfinder/blob/master/LICENSE) file.
