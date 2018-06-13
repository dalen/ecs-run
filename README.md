# ecs-run

Run a task on AWS ECS and stream output

## Usage

```
ecs-run 0.1.0
Erik Dalén <erik.gustav.dalen@gmail.com>

USAGE:
    ecs-run [OPTIONS] <CLUSTER> <SERVICE> <COMMAND>...

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -n, --name <CONTAINER>    Name of container to run command in

ARGS:
    <CLUSTER>       Name of cluster to run in
    <SERVICE>       Service to base task on
    <COMMAND>...    Command to run
```
