# pathfinder
WebSocket-over-RabbitMQ reverse proxy

# Usage
```
USAGE:
    pathfinder [FLAGS] [OPTIONS]

FLAGS:
    -h, --help        Prints help information
    -s, --secured     Enable creating a SSL connection RabbitMQ
    -v, --validate    Validate a token that was specified with data
    -V, --version     Prints version information

OPTIONS:
    -c, --config <config>                                  Path to a custom settings file [default: ]
    -i, --ip <ip>                                          The used IP for a server [default: 127.0.0.1]
        --jwt-secret <jwt_secret_key>                      Secret key for a JWT validation [default: secret]
    -l, --log-level <log_level>                            Verbosity level filter of the logger [default: info]
    -p, --port <port>                                      The listened port [default: 8080]
        --rabbitmq-ip <rabbitmq_ip>                        The used IP by RabbitMQ broker [default: 127.0.0.1]
        --rabbitmq-password <rabbitmq_password>            A RabbitMQ application password [default: password]
        --rabbitmq-port <rabbitmq_port>                    The listened port by RabbitMQ broker [default: 5672]
        --rabbitmq-user <rabbitmq_username>                A RabbitMQ application username [default: user]
        --rabbitmq-virtual-host <rabbitmq_virtual_host>    The virtual host of a RabbitMQ node [default: vhost]
        --redis-ip <redis_ip>                              The used IP by Redis [default: 127.0.0.1]
        --redis-password <redis_password>                  Password for connecting to redis [default: ]
        --redis-port <redis_port>                          The listened port by Redis [default: 6379]
        --ssl-cert <ssl_certificate>                       Path to a SSL certificate [default: ]
        --ssl-key <ssl_public_key>                         Path to a SSL public key [default: ]
```

# Configuration file
For using a custom configuration for reverse proxy, you will need to specify `-c` (or `--config`) option with a path to
a file. For example:
```bash
pathfinder --config=myconfig.yaml -p 8001
```
At the current stage of this project, reverse proxy is support only endpoints list, which is using for mapping URLs into certain Kafka topics.
Each of those endpoints contains two fields:
- `url` - URL that specified by a client in each request.
- `microservice` - Means the name of topic (or queue) where will be storing the message. This topic (or queue) is listening by certain microservice.

### Example
```yaml
endpoints:
  - search:
     url: "/api/matchmaking/search"
     microservice: "microservice.search"
  - leaderboard:
     url: "/api/matchmaking/leaderboard"
     microservice: "microservice.leaderboard"
```

# Documentation
Information about why this reverse proxy was implemented you can find [here](https://github.com/OpenMatchmaking/documentation/blob/master/docs/components.md#reverse-proxy).

# License
The pathfinder is published under BSD license. For more details read the [LICENSE](https://github.com/OpenMatchmaking/pathfinder/blob/master/LICENSE) file.
