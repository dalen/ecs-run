use clap::{App, Arg};
use tokio::runtime::Runtime;
use rusoto_core::Region;
use rusoto_ecs::{Ecs, EcsClient};
use rusoto_logs::{CloudWatchLogs, CloudWatchLogsClient};
use std::str::FromStr;
use std::{thread, time};

fn main() {
    let matches = App::new("ecs-run")
        .version("0.4.0")
        .author("Erik Dalén <erik.gustav.dalen@gmail.com>")
        .setting(clap::AppSettings::TrailingVarArg)
        .arg(
            Arg::with_name("VERBOSE")
                .short('v')
                .long("verbose")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::with_name("CONTAINER")
                .help("Name of container to run command in")
                .long("name")
                .short('n')
                .takes_value(true),
        )
        .arg(
            Arg::with_name("ENV")
                .help("Environment variable to pass to container, VAR=value")
                .long("env")
                .short('E')
                .multiple(true)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("CLUSTER")
                .help("Name of cluster to run in")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::with_name("SERVICE")
                .help("Service to base task on")
                .required(true)
                .index(2),
        )
        .arg(
            Arg::with_name("COMMAND")
                .help("Command to run")
                .required(true)
                .multiple(true)
                .index(3),
        )
        .arg(
            Arg::with_name("MEMORY")
                .long("memory")
                .short('m')
                .help("Overrides memory value for task and container, Memory value must be an increment in range <512, 30720> and be an increment of 1024")
                .required(false)
                .takes_value(true)
        )
        .get_matches();

    let cluster = matches.value_of("CLUSTER").unwrap();
    let service = matches.value_of("SERVICE").unwrap();
    let command = matches.values_of("COMMAND").unwrap();

    let env = matches.values_of("ENV");
    let verbose = matches.is_present("VERBOSE");
    let raw_memory =  matches.value_of("MEMORY");
    let memory: Option<i64>;

    if raw_memory.is_some() {
        memory = Option::from(raw_memory.unwrap().parse::<i64>().unwrap());
    } else {
        memory = None;
    }

    let container_name = matches.value_of("CONTAINER").unwrap();

    println!(
        "Running task: cluster: {cluster}, service: {service}, \
        container: {container_name}, memory:{memory}",
        container_name=container_name,
        memory = if raw_memory.is_some() { raw_memory.unwrap() } else { "None" }
    );

    let ecs_client = EcsClient::new(Region::default());
    match fetch_service(&ecs_client, &cluster, &service) {
        Ok(service) => {
            let task_definition = fetch_task_definition(&ecs_client, &service)
                .unwrap()
                .task_definition
                .unwrap();
            let container = get_container(&task_definition, Some(container_name));

            let log_options = container
                .clone()
                .log_configuration
                .unwrap()
                .options
                .unwrap();

            let task = run_task(
                &ecs_client,
                &cluster.to_string(),
                &service,
                &command.map(|s| s.to_string()).collect::<Vec<_>>(),
                parse_env(&env),
                &container,
                memory,
            );
            let task_id = &task
                .clone()
                .task_arn
                .unwrap()
                .rsplitn(2, '/')
                .next()
                .unwrap()
                .to_string();

            let mut previous_status = task.clone();
            println!("Started task {}", &task_id);
            loop {
                match fetch_task(&ecs_client, &cluster.to_string(), &task) {
                    // Task is likely not started yet, retry in a while
                    None => thread::sleep(time::Duration::from_millis(500)),
                    // Task was started, continue
                    Some(task_status) => {
                        if task_status.stopped_at != None {
                            break;
                        }

                        // Check if status has changed
                        if let (Some(ref old), Some(ref new)) =
                            (&task_status.last_status, &previous_status.last_status)
                        {
                            if old != new {
                                println!("Status: {}", new);
                            }
                        }
                        thread::sleep(time::Duration::from_millis(500));
                        previous_status = task_status;
                    }
                }
            }
            println!("Task finished, fetching logs");

            if !log_options.get("awslogs-group").is_none() {

                let log_group = log_options
                    .get("awslogs-group")
                    .expect("No log group configured");
                let log_region = log_options
                    .get("awslogs-region")
                    .expect("No log region configured");
                let log_prefix = log_options
                    .get("awslogs-stream-prefix")
                    .expect("No log stream prefix configured");

                if verbose {
                    println!("Creating logs client in region: {}", &log_region);
                }

                let logs_client = CloudWatchLogsClient::new(Region::from_str(&log_region).unwrap());
                let log_stream_name =
                    format!("{}/{}/{}", &log_prefix, &container.name.unwrap(), &task_id);

                if verbose {
                    println!(
                        "Fetching logs: group: {}, stream: {}",
                        &log_group.to_string(),
                        &log_stream_name.to_string()
                    );
                }
                let logs = fetch_logs(&logs_client, &log_group, &log_stream_name);

                for log in &logs.clone().events.unwrap() {
                    match &log.message {
                        Some(message) => println!("{}", &message),
                        None => (),
                    }
                }
            } else {
                println!("No Cloudwatch log configuration provided");
            }

            std::process::exit(get_exit_code(&previous_status) as i32);
        }
        Err(error) => {
            println!("Error: {}", error);
        }
    }
}

// Parse out the environment variables from options and return them in
// a format that rusoto expects
fn parse_env(env_matches: &Option<clap::Values>) -> Option<Vec<rusoto_ecs::KeyValuePair>> {
    env_matches.clone().map(|envs| {
        envs.map(|env| {
            let mut parts = env.splitn(2, '=');
            rusoto_ecs::KeyValuePair {
                name: parts.next().map(|s| s.to_string()),
                value: parts.next().map(|s| s.to_string()),
            }
        })
        .collect()
    })
}

// TODO: loop if there are more logs
fn fetch_logs(
    client: &rusoto_logs::CloudWatchLogsClient,
    log_group_name: &str,
    log_stream_name: &str,
) -> rusoto_logs::GetLogEventsResponse {
    let runtime = Runtime::new().unwrap();

    let result = client
        .get_log_events(rusoto_logs::GetLogEventsRequest {
            log_group_name: log_group_name.to_string(),
            log_stream_name: log_stream_name.to_string(),
            ..Default::default()
        });
    runtime.block_on(result).unwrap()
}

fn fetch_task(
    client: &EcsClient,
    cluster: &str,
    task: &rusoto_ecs::Task,
) -> Option<rusoto_ecs::Task> {
    let task_arn = task.clone().task_arn.unwrap();
    let runtime = Runtime::new().unwrap();

    let result = runtime.block_on(
        client.describe_tasks(rusoto_ecs::DescribeTasksRequest {
            cluster: Some(cluster.to_string()),
            tasks: vec![task_arn.clone()],
            include: None,
        })
    );

    let tasks = result
        .unwrap()
        .tasks
        .expect("Task definition response contained no tasks");
    if tasks.is_empty() {
        None
    } else {
        Some(tasks[0].clone())
    }
}

// Get the exit code
fn get_exit_code(task: &rusoto_ecs::Task) -> i64 {
    match &task.containers {
        Some(containers) => {
            for container in containers.clone() {
                match &container.exit_code {
                    Some(exit_code) => {
                        if *exit_code != 0 {
                            return *exit_code;
                        }
                    }
                    None => {}
                }
            }
            0
        }
        None => 0,
    }
}

// Get container with matching name if one is supplied
fn get_container(
    task_definition: &rusoto_ecs::TaskDefinition,
    name: Option<&str>,
) -> rusoto_ecs::ContainerDefinition {
    let containers = task_definition
        .clone()
        .container_definitions
        .unwrap_or_default();

    match name {
        Some(n) => containers
            .iter()
            .find(|c| c.name == Some(n.to_string()))
            .unwrap_or_else(|| panic!("No container called {} found in task", &n))
            .clone(),
        None => {
            if containers.len() != 1 {
                panic!("Task has more than one container and which one to run in was not specified with the -n flag.");
            } else {
                containers[0].clone()
            }
        }
    }
}

fn run_task(
    client: &EcsClient,
    cluster: &str,
    service: &rusoto_ecs::Service,
    command: &[String],
    env: Option<Vec<rusoto_ecs::KeyValuePair>>,
    container: &rusoto_ecs::ContainerDefinition,
    memory: Option<i64>,
) -> rusoto_ecs::Task {
    let runtime = Runtime::new().unwrap();
    let service = service.clone();
    let container_memory: Option<i64>;
    let task_memory: Option<String>;

    if memory.is_some() {
        let memory = memory.unwrap();
        container_memory = Option::from(memory - 512);
        task_memory = Option::from(memory.to_string());
    } else {
        container_memory = None;
        task_memory = None;
    }
    let result = runtime.block_on(client
        .run_task(rusoto_ecs::RunTaskRequest {
            cluster: Some(cluster.to_string()),
            count: Some(1),
            launch_type: service.launch_type,
            network_configuration: service.network_configuration,
            placement_constraints: service.placement_constraints,
            placement_strategy: service.placement_strategy,
            platform_version: service.platform_version,
            task_definition: service
                .task_definition
                .expect("No task definition in service"),
            overrides: Some(rusoto_ecs::TaskOverride {
                container_overrides: Some(vec![rusoto_ecs::ContainerOverride {
                    name: container.name.clone(),
                    command: Some(command.to_vec()),
                    environment: env,
                    memory: container_memory, // Leave some memory for other containers
                    ..Default::default()
                }]),
                memory: task_memory,
                ..Default::default()
            }),
            started_by: Some("ecs-run".to_string()),
            ..Default::default()
        }));
    let tasks = result
        .unwrap()
        .tasks
        .expect("run_task response contained no tasks");

    if tasks.is_empty() {
        panic!("No tasks were started by run_task")
    } else {
        tasks[0].clone()
    }
}

fn fetch_task_definition(
    client: &EcsClient,
    service: &rusoto_ecs::Service,
) -> Result<
    rusoto_ecs::DescribeTaskDefinitionResponse,
    rusoto_core::RusotoError<rusoto_ecs::DescribeTaskDefinitionError>,
> {
    let runtime = Runtime::new().unwrap();
    runtime.block_on(
        client.
            describe_task_definition(rusoto_ecs::DescribeTaskDefinitionRequest {
            task_definition: service.clone().task_definition.unwrap(),
            include: None,
        })
    )
}

fn fetch_service(
    client: &EcsClient,
    cluster: &str,
    service: &str,
) -> Result<rusoto_ecs::Service, String> {
    let runtime = Runtime::new().unwrap();
    match runtime.block_on(
        client
        .describe_services(rusoto_ecs::DescribeServicesRequest {
            cluster: Some(cluster.to_string()),
            services: vec![service.to_string()],
            include: None,
        })
    )
    {
        Ok(response) => match response.services {
            Some(services) => {
                if services.is_empty() {
                    Err(format!("Could not find service {}", &service))
                } else {
                    Ok(services[0].clone())
                }
            }
            None => Err(format!("Could not find service {}", &service)),
        },
        Err(error) => Err(format!("Error: {:?}", &error)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_env() {
        let m = App::new("myapp")
            .arg(
                Arg::with_name("env")
                    .short('E')
                    .multiple(true)
                    .takes_value(true),
            )
            .get_matches_from(vec!["myapp", "-E", "FOO=bar"]);

        let values = m.values_of("env");

        assert_eq!(
            parse_env(&values),
            Some(vec![rusoto_ecs::KeyValuePair {
                name: Some(String::from("FOO")),
                value: Some(String::from("bar")),
            }])
        )
    }
}
