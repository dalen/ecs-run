# ecs-run

Run a task on AWS ECS and stream output.

As input this takes an existing cluster and service and runs a task with the same subnets and execution role, but with a different command.
This can for example be used to run a rake task on a Rails service in ECS.

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

## Related work

- [ecs-run-task](https://github.com/buildkite/ecs-run-task) - Similar, but creates a new task definition instead of reusing the one from an existing service.
