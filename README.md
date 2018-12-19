# pathfinder
An asynchronous WebSocket-over-RabbitMQ reverse proxy, based on the tokio and futures-rs crates.

# Features
- Configuring a behaviour of the reverse proxy via CLI options and YAML files
- Communicating with Auth/Auth microservice for validating a JSON Web Token, getting a list of permissions before getting an access to other microservices
- Transferring requests to the certain microservices via RabbitMQ queues and returning responses in JSON format

# Usage
```
USAGE:
    pathfinder [FLAGS] [OPTIONS]

FLAGS:
    -s, --secured     Enable the SSL/TLS mode for connections with RabbitMQ
    -h, --help        Prints help information
    -V, --version     Prints version information

OPTIONS:
    -c, --config <config>                                  Path to a custom settings file [default: ]
    -i, --ip <ip>                                          The used IP for a server [default: 127.0.0.1]
    -p, --port <port>                                      The listened port [default: 9000]
    -l, --log-level <log_level>                            Verbosity level filter of the logger [default: info]
        --rabbitmq-host <rabbitmq_host>                    The used host by RabbitMQ broker [default: 127.0.0.1]
        --rabbitmq-port <rabbitmq_port>                    The listened port by RabbitMQ broker [default: 5672]
        --rabbitmq-virtual-host <rabbitmq_virtual_host>    The virtual host of a RabbitMQ node [default: vhost]
        --rabbitmq-user <rabbitmq_username>                A RabbitMQ application username [default: user]
        --rabbitmq-password <rabbitmq_password>            A RabbitMQ application password [default: password]
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
Each of those endpoints contains four fields:
- `url` - URL that specified by a client in each request. Required.
- `routing_key` - Means the name of topic (or queue) where will be storing the message. This topic (or queue) is listening by certain microservice. Required.
- `request_exchange` - Defines the name of exchange point for RabbitMQ, through which the reverse proxy should publish a message. Optional. Default: `"open-matchmaking.direct"`
- `response_exchange` - Defines the name of exchange point for RabbitMQ, through which the reverse proxy should consume a message. Optional. Default: `"open-matchmaking.responses.direct"`
- `token_required` - Defines does the endpoint need any extra checks for credentials before getting an access to it. Optional. Default: `true`.

### Example
```yaml
endpoints:
  - search:
      url: "/api/matchmaking/search"
      routing_key: "microservice.search"
  - leaderboard:
      url: "/api/matchmaking/leaderboard"
      routing_key: "microservice.leaderboard"
      request_exchange: "amqp.direct"
      response_exchange:  "open-matchmaking.default.direct"
```

# Documentation
Information about why this reverse proxy was implemented you can find [here](https://github.com/OpenMatchmaking/documentation/blob/master/docs/components/reverse-proxy.md#reverse-proxy).

# Benchmarks
For performance benchmark was used the *MacBook Pro 13" (mid 2012)* with *2,5Ghz Intel Core i5 (2x cores, 4 threads)* processor and 16Gb memory. The tests were running in the following conditions:
1. All required microservices and external resources were running via Docker containers.
2. The test cluster was using 12 of 16Gb of available memory and all available cores / threads from my notebook.
3. During the tests were simulated 1000 concurrent users via Gatling tool, with splitting the traffic onto 5 stages so that it gradually increased.

The test were made in two passes:
1. Relies on processing requests without validating tokens and passing the data as is to the Echo microservice.
2. Uses an information that necessary to register and to generate a token for getting an access to microservice:
- Communicating with Auth/Auth for registering a new user and generating JSON Web Token
- Token from the previous step must be used with data for getting an access to the Echo microservice. During this simulation step the token will be verified by the Auth/Auth microservice before passing a requests further.

The repository with benchmarks can be found [here](https://github.com/OpenMatchmaking/bench-pathfinder).

| Metric name \ Test name    | Without token | With Json Web Token (JWT) | 
|----------------------------|---------------|---------------------------| 
| Startup RAM usage, Mb      | 2.21          | 2.23                      |
| Max RAM usage, Mb          | 69.54         | 66.48                     |
| Avg RAM usage, Mb          | 42.02         | 48.54                     |
| Max CPU usage, %           | 114.04        | 120.398                   |
| Avg CPU usage, %           | 69.54         | 45.66                     |
| Total requests             | 6075          | 2241                      |
| Successfully processed     | 6075          | 2241                      |
| Error responses            | 0             | 0                         |
| Min response time (ms)     | 3             | 15                        |
| Max response time (ms)     | 13243         | 38945                     |
| Mean response time (ms)    | 2450          | 8948                      |
| Std dev response time (ms) | 3933          | 13833                     |
| 50th percentile (ms)       | 17            | 25                        |
| 75th percentile (ms)       | 4824          | 20372                     |
| 95th percentile (ms)       | 11358         | 37672                     |
| 99th percentile (ms)       | 12968         | 38587                     |
| Requests / sec             | 162.723       | 53.357                    |

*NOTE: Keep in mind the response time and RPS (requests per seconds) are much lower on the second pass because necessary to communicate with Auth/Auth microservice a couple of times before doing an actual work.*

# License
The pathfinder is published under BSD license. For more details read the [LICENSE](https://github.com/OpenMatchmaking/pathfinder/blob/master/LICENSE) file.
