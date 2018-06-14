[![Build Status](https://travis-ci.com/dalen/ecs-run.svg?branch=master)](https://travis-ci.com/dalen/ecs-run)

# ecs-run

Run a task on AWS ECS and stream output.

As input this takes an existing cluster and service and runs a task with the same subnets and execution role, but with a different command.
This can for example be used to run a rake task on a Rails service in ECS.

## Usage

```
ecs-run 0.2.0
Erik Dalén <erik.gustav.dalen@gmail.com>

USAGE:
    ecs-run [OPTIONS] <CLUSTER> <SERVICE> <COMMAND>...

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -n, --name <CONTAINER>    Name of container to run command in
    -E, --env <ENV>...        Environment variable to pass to container, VAR=value

ARGS:
    <CLUSTER>       Name of cluster to run in
    <SERVICE>       Service to base task on
    <COMMAND>...    Command to run
```

### Using docker

This can be run using Docker in the folloging way:

```
docker run -it -v ~/.aws:/root/.aws:ro -e AWS_PROFILE -e AWS_REGION -e AWS_DEFAULT_REGION edalen/ecs-run <cluster> <service> <command>
```

If you have the credentials set as environment variables you might need to forward them as well, by adding `-e AWS_ACCESS_KEY_ID -e AWS_SECRET_ACCESS_KEY` etc.

If you run it in CodeBuild you have to pass the `AWS_CONTAINER_CREDENTIALS_RELATIVE_URI` to give it access to the IAM role.

## Related work

- [ecs-run-task](https://github.com/buildkite/ecs-run-task) - Similar, but creates a new task definition instead of reusing the one from an existing service.
