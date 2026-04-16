use std::ffi::OsString;
use std::path::PathBuf;

use serde_json::Value;

use crate::error::EngineError;
use crate::runner::{CommandOutput, CommandRunner, SystemCommandRunner};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrimitiveResult {
    pub status: i32,
    pub stdout: String,
    pub stderr: String,
}

impl From<CommandOutput> for PrimitiveResult {
    fn from(output: CommandOutput) -> Self {
        Self { status: output.status, stdout: output.stdout, stderr: output.stderr }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContainerInspectResult {
    pub raw: Value,
    pub transport: PrimitiveResult,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContainerBindMount {
    pub source: PathBuf,
    pub target: PathBuf,
    pub readonly: bool,
}

impl ContainerBindMount {
    pub fn new(source: impl Into<PathBuf>, target: impl Into<PathBuf>) -> Self {
        Self { source: source.into(), target: target.into(), readonly: false }
    }

    pub fn readonly(mut self, readonly: bool) -> Self {
        self.readonly = readonly;
        self
    }
}

#[derive(Debug, Clone, Default)]
pub struct BuildRequest {
    pub context: PathBuf,
    pub dockerfile: Option<PathBuf>,
    pub tag: Option<String>,
    pub build_args: Vec<(String, String)>,
    pub labels: Vec<(String, String)>,
    pub no_cache: bool,
}

#[derive(Debug, Clone, Default)]
pub struct ContainerCreateRequest {
    pub image: String,
    pub name: Option<String>,
    pub env: Vec<(String, String)>,
    pub labels: Vec<(String, String)>,
    pub mounts: Vec<ContainerBindMount>,
    pub workdir: Option<String>,
    pub user: Option<String>,
    pub entrypoint: Option<String>,
    pub command: Option<Vec<String>>,
    pub tty: bool,
    pub interactive: bool,
    pub detach: bool,
    pub remove: bool,
}

#[derive(Debug, Clone, Default)]
pub struct ExecRequest {
    pub container: String,
    pub command: Vec<String>,
    pub env: Vec<(String, String)>,
    pub labels: Vec<(String, String)>,
    pub workdir: Option<String>,
    pub user: Option<String>,
    pub tty: bool,
    pub interactive: bool,
    pub detach: bool,
    pub privileged: bool,
    pub remove: bool,
}

#[derive(Debug, Clone, Default)]
pub struct ComposeProjectRequest {
    pub files: Vec<PathBuf>,
    pub project_directory: Option<PathBuf>,
    pub project_name: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ComposeUpRequest {
    pub project: ComposeProjectRequest,
    pub services: Vec<String>,
    pub detach: bool,
    pub build: bool,
    pub no_build: bool,
    pub force_recreate: bool,
    pub no_recreate: bool,
    pub remove_orphans: bool,
    pub wait: bool,
}

#[derive(Debug, Clone, Default)]
pub struct ComposeBuildRequest {
    pub files: Vec<PathBuf>,
    pub service: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ComposePsRequest {
    pub project: ComposeProjectRequest,
    pub services: Vec<String>,
    pub all: bool,
    pub quiet: bool,
    pub format_json: bool,
}

#[derive(Debug, Clone, Default)]
pub struct ComposeExecRequest {
    pub project: ComposeProjectRequest,
    pub service: String,
    pub command: Vec<String>,
    pub env: Vec<(String, String)>,
    pub workdir: Option<String>,
    pub user: Option<String>,
    pub index: Option<u32>,
    pub tty: bool,
    pub detach: bool,
    pub privileged: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComposePsResult {
    pub raw: Value,
    pub transport: PrimitiveResult,
}

#[derive(Debug, Clone, Default)]
pub struct ComposeTargetResolutionRequest {
    pub project: ComposeProjectRequest,
    pub service: String,
    pub prefer_running: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComposeTargetResolutionResult {
    pub service: String,
    pub project_name: Option<String>,
    pub container_id: Option<String>,
    pub container_name: Option<String>,
    pub state: Option<String>,
    pub status: Option<String>,
    pub raw: Value,
    pub transport: PrimitiveResult,
}

#[derive(Debug, Clone)]
pub struct DockerEngine<R = SystemCommandRunner> {
    docker_binary: OsString,
    runner: R,
}

impl Default for DockerEngine<SystemCommandRunner> {
    fn default() -> Self {
        Self::new()
    }
}

impl DockerEngine<SystemCommandRunner> {
    pub fn new() -> Self {
        Self { docker_binary: OsString::from("docker"), runner: SystemCommandRunner }
    }
}

impl<R: CommandRunner> DockerEngine<R> {
    pub fn with_runner(runner: R) -> Self {
        Self { docker_binary: OsString::from("docker"), runner }
    }

    pub fn with_binary_and_runner(binary: impl Into<OsString>, runner: R) -> Self {
        Self { docker_binary: binary.into(), runner }
    }

    pub fn inspect(&self, reference: &str) -> Result<ContainerInspectResult, EngineError> {
        let output = self.run([OsString::from("inspect"), OsString::from(reference)])?;
        let raw: Value = serde_json::from_str(&output.stdout).map_err(|_source| {
            EngineError::InvalidInspectOutput {
                message: "docker inspect did not return valid JSON".to_string(),
                stdout: output.stdout.clone(),
                stderr: output.stderr.clone(),
            }
        })?;
        let raw = match raw {
            Value::Array(mut items) if items.len() == 1 => items.remove(0),
            Value::Array(_) => {
                return Err(EngineError::InvalidInspectOutput {
                    message: "docker inspect returned more than one document".to_string(),
                    stdout: output.stdout.clone(),
                    stderr: output.stderr.clone(),
                });
            }
            value => value,
        };

        Ok(ContainerInspectResult { raw, transport: to_primitive_result(output) })
    }

    pub fn build(&self, request: BuildRequest) -> Result<PrimitiveResult, EngineError> {
        let mut args = vec![OsString::from("build")];
        if let Some(dockerfile) = request.dockerfile {
            args.push(OsString::from("-f"));
            args.push(dockerfile.into_os_string());
        }
        if let Some(tag) = request.tag {
            args.push(OsString::from("-t"));
            args.push(OsString::from(tag));
        }
        if request.no_cache {
            args.push(OsString::from("--no-cache"));
        }
        for (key, value) in request.build_args {
            args.push(OsString::from("--build-arg"));
            args.push(OsString::from(format!("{key}={value}")));
        }
        for (key, value) in request.labels {
            args.push(OsString::from("--label"));
            args.push(OsString::from(format!("{key}={value}")));
        }
        args.push(request.context.into_os_string());

        self.run(args).map(to_primitive_result)
    }

    pub fn create(&self, request: ContainerCreateRequest) -> Result<PrimitiveResult, EngineError> {
        let mut args = vec![OsString::from("create")];
        if let Some(name) = request.name {
            args.push(OsString::from("--name"));
            args.push(OsString::from(name));
        }
        if request.detach {
            args.push(OsString::from("-d"));
        }
        if request.interactive {
            args.push(OsString::from("-i"));
        }
        if request.tty {
            args.push(OsString::from("-t"));
        }
        if request.remove {
            args.push(OsString::from("--rm"));
        }
        if let Some(workdir) = request.workdir {
            args.push(OsString::from("--workdir"));
            args.push(OsString::from(workdir));
        }
        if let Some(user) = request.user {
            args.push(OsString::from("--user"));
            args.push(OsString::from(user));
        }
        for (key, value) in request.env {
            args.push(OsString::from("--env"));
            args.push(OsString::from(format!("{key}={value}")));
        }
        for (key, value) in request.labels {
            args.push(OsString::from("--label"));
            args.push(OsString::from(format!("{key}={value}")));
        }
        for mount in request.mounts {
            args.push(OsString::from("--mount"));
            args.push(OsString::from(format_bind_mount(&mount)));
        }
        if let Some(entrypoint) = request.entrypoint {
            args.push(OsString::from("--entrypoint"));
            args.push(OsString::from(entrypoint));
        }
        args.push(OsString::from(request.image));
        if let Some(command) = request.command {
            args.extend(command.into_iter().map(OsString::from));
        }

        self.run(args).map(to_primitive_result)
    }

    pub fn start(&self, container: &str) -> Result<PrimitiveResult, EngineError> {
        self.run([OsString::from("start"), OsString::from(container)]).map(to_primitive_result)
    }

    pub fn exec(&self, request: ExecRequest) -> Result<PrimitiveResult, EngineError> {
        if request.command.is_empty() {
            return Err(EngineError::InvalidRequest {
                message: "exec requires a command".to_string(),
            });
        }

        let mut args = vec![OsString::from("exec")];
        if request.detach {
            args.push(OsString::from("-d"));
        }
        if request.interactive {
            args.push(OsString::from("-i"));
        }
        if request.tty {
            args.push(OsString::from("-t"));
        }
        if request.privileged {
            args.push(OsString::from("--privileged"));
        }
        if request.remove {
            args.push(OsString::from("--rm"));
        }
        if let Some(workdir) = request.workdir {
            args.push(OsString::from("--workdir"));
            args.push(OsString::from(workdir));
        }
        if let Some(user) = request.user {
            args.push(OsString::from("--user"));
            args.push(OsString::from(user));
        }
        for (key, value) in request.env {
            args.push(OsString::from("--env"));
            args.push(OsString::from(format!("{key}={value}")));
        }
        for (key, value) in request.labels {
            args.push(OsString::from("--label"));
            args.push(OsString::from(format!("{key}={value}")));
        }
        args.push(OsString::from(request.container));
        args.extend(request.command.into_iter().map(OsString::from));

        self.run(args).map(to_primitive_result)
    }

    pub fn compose_build(
        &self,
        request: ComposeBuildRequest,
    ) -> Result<PrimitiveResult, EngineError> {
        let mut args = compose_file_args(request.files);
        args.push(OsString::from("build"));
        if let Some(service) = request.service {
            args.push(OsString::from(service));
        }
        self.run(args).map(to_primitive_result)
    }

    pub fn compose_up(&self, request: ComposeUpRequest) -> Result<PrimitiveResult, EngineError> {
        let mut args = compose_base_args(&request.project);
        args.push(OsString::from("up"));
        if request.detach {
            args.push(OsString::from("-d"));
        }
        if request.build {
            args.push(OsString::from("--build"));
        }
        if request.no_build {
            args.push(OsString::from("--no-build"));
        }
        if request.force_recreate {
            args.push(OsString::from("--force-recreate"));
        }
        if request.no_recreate {
            args.push(OsString::from("--no-recreate"));
        }
        if request.remove_orphans {
            args.push(OsString::from("--remove-orphans"));
        }
        if request.wait {
            args.push(OsString::from("--wait"));
        }
        args.extend(request.services.into_iter().map(OsString::from));
        self.run(args).map(to_primitive_result)
    }

    pub fn compose_exec(
        &self,
        request: ComposeExecRequest,
    ) -> Result<PrimitiveResult, EngineError> {
        if request.command.is_empty() {
            return Err(EngineError::InvalidRequest {
                message: "compose exec requires a command".to_string(),
            });
        }

        let mut args = compose_base_args(&request.project);
        args.push(OsString::from("exec"));
        if request.detach {
            args.push(OsString::from("-d"));
        }
        if !request.tty {
            args.push(OsString::from("-T"));
        }
        if let Some(index) = request.index {
            args.push(OsString::from("--index"));
            args.push(OsString::from(index.to_string()));
        }
        if request.privileged {
            args.push(OsString::from("--privileged"));
        }
        if let Some(workdir) = request.workdir {
            args.push(OsString::from("--workdir"));
            args.push(OsString::from(workdir));
        }
        if let Some(user) = request.user {
            args.push(OsString::from("--user"));
            args.push(OsString::from(user));
        }
        for (key, value) in request.env {
            args.push(OsString::from("--env"));
            args.push(OsString::from(format!("{key}={value}")));
        }
        args.push(OsString::from(request.service));
        args.extend(request.command.into_iter().map(OsString::from));
        self.run(args).map(to_primitive_result)
    }

    pub fn compose_ps(&self, request: ComposePsRequest) -> Result<ComposePsResult, EngineError> {
        let mut args = compose_base_args(&request.project);
        args.push(OsString::from("ps"));
        if request.all {
            args.push(OsString::from("-a"));
        }
        if request.quiet {
            args.push(OsString::from("-q"));
        }
        if request.format_json {
            args.push(OsString::from("--format"));
            args.push(OsString::from("json"));
        }
        args.extend(request.services.into_iter().map(OsString::from));

        let output = self.run(args)?;
        let raw: Value = serde_json::from_str(&output.stdout).map_err(|_source| {
            EngineError::InvalidComposeOutput {
                message: "docker compose ps did not return valid JSON".to_string(),
                stdout: output.stdout.clone(),
                stderr: output.stderr.clone(),
            }
        })?;

        Ok(ComposePsResult { raw, transport: output.into() })
    }

    pub fn resolve_compose_target(
        &self,
        request: ComposeTargetResolutionRequest,
    ) -> Result<ComposeTargetResolutionResult, EngineError> {
        let ps = self.compose_ps(ComposePsRequest {
            project: request.project.clone(),
            services: vec![request.service.clone()],
            all: true,
            quiet: false,
            format_json: true,
        })?;
        let entries = ps.raw.as_array().ok_or_else(|| EngineError::InvalidComposeOutput {
            message: "docker compose ps JSON was not an array".to_string(),
            stdout: ps.transport.stdout.clone(),
            stderr: ps.transport.stderr.clone(),
        })?;

        let selected = select_compose_entry(entries, &request.service, request.prefer_running)
            .ok_or_else(|| EngineError::InvalidComposeOutput {
                message: format!("no compose target found for service `{}`", request.service),
                stdout: ps.transport.stdout.clone(),
                stderr: ps.transport.stderr.clone(),
            })?;

        Ok(ComposeTargetResolutionResult {
            service: request.service,
            project_name: compose_string_field(selected, &["Project", "project"])
                .map(str::to_owned),
            container_id: compose_string_field(selected, &["ID", "Id", "id"]).map(str::to_owned),
            container_name: compose_string_field(selected, &["Name", "name"]).map(str::to_owned),
            state: compose_string_field(selected, &["State", "state"]).map(str::to_owned),
            status: compose_string_field(selected, &["Status", "status"]).map(str::to_owned),
            raw: selected.clone(),
            transport: ps.transport,
        })
    }

    pub(crate) fn run<I>(&self, args: I) -> Result<CommandOutput, EngineError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let args: Vec<OsString> = args.into_iter().collect();
        let output = self.runner.run(&self.docker_binary, &args)?;
        if output.status != 0 {
            return Err(EngineError::Exit {
                program: self.docker_binary.to_string_lossy().into_owned(),
                status: output.status,
                stdout: output.stdout,
                stderr: output.stderr,
            });
        }

        Ok(output)
    }
}

fn to_primitive_result(output: CommandOutput) -> PrimitiveResult {
    output.into()
}

fn compose_base_args(project: &ComposeProjectRequest) -> Vec<OsString> {
    let mut args = vec![OsString::from("compose")];
    for file in &project.files {
        args.push(OsString::from("-f"));
        args.push(file.clone().into_os_string());
    }
    if let Some(project_directory) = &project.project_directory {
        args.push(OsString::from("--project-directory"));
        args.push(project_directory.clone().into_os_string());
    }
    if let Some(project_name) = &project.project_name {
        args.push(OsString::from("--project-name"));
        args.push(OsString::from(project_name));
    }
    args
}

fn compose_file_args(files: Vec<PathBuf>) -> Vec<OsString> {
    let mut args = vec![OsString::from("compose")];
    for file in files {
        args.push(OsString::from("-f"));
        args.push(file.into_os_string());
    }
    args
}

fn select_compose_entry<'a>(
    entries: &'a [Value],
    service: &str,
    prefer_running: bool,
) -> Option<&'a Value> {
    let matching: Vec<&Value> = entries
        .iter()
        .filter(|entry| {
            compose_string_field(entry, &["Service", "service", "Name", "name"]) == Some(service)
        })
        .collect();
    if matching.is_empty() {
        return None;
    }

    if prefer_running {
        if let Some(entry) = matching.iter().find(|entry| {
            let state = compose_string_field(entry, &["State", "state"]).unwrap_or_default();
            let status = compose_string_field(entry, &["Status", "status"]).unwrap_or_default();
            state.eq_ignore_ascii_case("running") || status.to_ascii_lowercase().contains("running")
        }) {
            return Some(*entry);
        }
    }

    matching.into_iter().next()
}

fn compose_string_field<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a str> {
    let object = value.as_object()?;
    for key in keys {
        if let Some(value) = object.get(*key).and_then(Value::as_str) {
            return Some(value);
        }
    }
    None
}

fn format_bind_mount(mount: &ContainerBindMount) -> String {
    let mut parts = vec![
        "type=bind".to_string(),
        format!("source={}", mount.source.to_string_lossy()),
        format!("target={}", mount.target.to_string_lossy()),
    ];
    if mount.readonly {
        parts.push("readonly".to_string());
    }
    parts.join(",")
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::ffi::{OsStr, OsString};

    use super::*;
    use crate::runner::{CommandInvocation, CommandRunner};

    #[derive(Default)]
    struct FakeRunner {
        invocations: RefCell<Vec<CommandInvocation>>,
        responses: RefCell<Vec<Result<CommandOutput, EngineError>>>,
    }

    impl FakeRunner {
        fn push_response(&self, response: Result<CommandOutput, EngineError>) {
            self.responses.borrow_mut().push(response);
        }

        fn invocations(&self) -> Vec<CommandInvocation> {
            self.invocations.borrow().clone()
        }
    }

    impl CommandRunner for FakeRunner {
        fn run(&self, program: &OsStr, args: &[OsString]) -> Result<CommandOutput, EngineError> {
            self.invocations
                .borrow_mut()
                .push(CommandInvocation { program: program.to_os_string(), args: args.to_vec() });
            self.responses.borrow_mut().remove(0)
        }
    }

    impl CommandRunner for std::rc::Rc<FakeRunner> {
        fn run(&self, program: &OsStr, args: &[OsString]) -> Result<CommandOutput, EngineError> {
            self.as_ref().run(program, args)
        }
    }

    #[test]
    fn inspect_parses_json_and_records_command() {
        let runner = std::rc::Rc::new(FakeRunner::default());
        runner.push_response(Ok(CommandOutput {
            status: 0,
            stdout: r#"[{"Id":"abc","State":{"Status":"running"}}]"#.to_string(),
            stderr: String::new(),
        }));

        let engine = DockerEngine::with_binary_and_runner("docker", runner.clone());
        let result = engine.inspect("abc").expect("inspect result");

        assert_eq!(result.raw["Id"], "abc");
        assert_eq!(result.transport.status, 0);
        assert_eq!(
            runner.invocations(),
            vec![CommandInvocation {
                program: OsString::from("docker"),
                args: vec![OsString::from("inspect"), OsString::from("abc")],
            }]
        );
    }

    #[test]
    fn build_formats_flags_in_expected_order() {
        let runner = std::rc::Rc::new(FakeRunner::default());
        runner.push_response(Ok(CommandOutput {
            status: 0,
            stdout: "built".to_string(),
            stderr: String::new(),
        }));

        let engine = DockerEngine::with_binary_and_runner("docker", runner.clone());
        let result = engine
            .build(BuildRequest {
                context: PathBuf::from("."),
                dockerfile: Some(PathBuf::from("Dockerfile")),
                tag: Some("deco:test".to_string()),
                build_args: vec![("KEY".to_string(), "VALUE".to_string())],
                labels: vec![("app".to_string(), "deco".to_string())],
                no_cache: true,
            })
            .expect("build result");

        assert_eq!(result.stdout, "built");
        assert_eq!(
            runner.invocations(),
            vec![CommandInvocation {
                program: OsString::from("docker"),
                args: vec![
                    OsString::from("build"),
                    OsString::from("-f"),
                    OsString::from("Dockerfile"),
                    OsString::from("-t"),
                    OsString::from("deco:test"),
                    OsString::from("--no-cache"),
                    OsString::from("--build-arg"),
                    OsString::from("KEY=VALUE"),
                    OsString::from("--label"),
                    OsString::from("app=deco"),
                    OsString::from("."),
                ],
            }]
        );
    }

    #[test]
    fn errors_map_into_engine_taxonomy() {
        let runner = std::rc::Rc::new(FakeRunner::default());
        runner.push_response(Ok(CommandOutput {
            status: 1,
            stdout: String::new(),
            stderr: "boom".to_string(),
        }));

        let engine = DockerEngine::with_binary_and_runner("docker", runner.clone());
        let error = engine.start("abc").expect_err("start should fail on non-zero exit");

        let deco_error: deco_core_model::DecoError = error.into();
        assert_eq!(deco_error.category, deco_core_model::ErrorCategory::Engine);
        assert!(deco_error.message.contains("exited with status 1"));
    }

    #[test]
    fn create_formats_container_options() {
        let runner = std::rc::Rc::new(FakeRunner::default());
        runner.push_response(Ok(CommandOutput {
            status: 0,
            stdout: "created".to_string(),
            stderr: String::new(),
        }));

        let engine = DockerEngine::with_binary_and_runner("docker", runner.clone());
        engine
            .create(ContainerCreateRequest {
                image: "alpine:3.20".to_string(),
                name: Some("deco".to_string()),
                env: vec![("A".to_string(), "B".to_string())],
                labels: vec![("k".to_string(), "v".to_string())],
                mounts: vec![
                    ContainerBindMount::new("/host/workspace", "/workspace").readonly(true),
                ],
                workdir: Some("/workspace".to_string()),
                user: Some("1000:1000".to_string()),
                entrypoint: Some("/bin/sh -lc".to_string()),
                command: Some(vec!["echo".to_string(), "hi".to_string()]),
                tty: true,
                interactive: true,
                detach: true,
                remove: true,
            })
            .expect("create result");

        assert_eq!(
            runner.invocations(),
            vec![CommandInvocation {
                program: OsString::from("docker"),
                args: vec![
                    OsString::from("create"),
                    OsString::from("--name"),
                    OsString::from("deco"),
                    OsString::from("-d"),
                    OsString::from("-i"),
                    OsString::from("-t"),
                    OsString::from("--rm"),
                    OsString::from("--workdir"),
                    OsString::from("/workspace"),
                    OsString::from("--user"),
                    OsString::from("1000:1000"),
                    OsString::from("--env"),
                    OsString::from("A=B"),
                    OsString::from("--label"),
                    OsString::from("k=v"),
                    OsString::from("--mount"),
                    OsString::from("type=bind,source=/host/workspace,target=/workspace,readonly"),
                    OsString::from("--entrypoint"),
                    OsString::from("/bin/sh -lc"),
                    OsString::from("alpine:3.20"),
                    OsString::from("echo"),
                    OsString::from("hi"),
                ],
            }]
        );
    }

    #[test]
    fn compose_up_formats_expected_arguments() {
        let runner = std::rc::Rc::new(FakeRunner::default());
        runner.push_response(Ok(CommandOutput {
            status: 0,
            stdout: "started".to_string(),
            stderr: String::new(),
        }));

        let engine = DockerEngine::with_binary_and_runner("docker", runner.clone());
        engine
            .compose_up(ComposeUpRequest {
                project: ComposeProjectRequest {
                    files: vec![
                        PathBuf::from("compose.yml"),
                        PathBuf::from("compose.override.yml"),
                    ],
                    project_directory: Some(PathBuf::from("/workspace")),
                    project_name: Some("deco".to_string()),
                },
                services: vec!["app".to_string(), "db".to_string()],
                detach: true,
                build: true,
                force_recreate: true,
                remove_orphans: true,
                wait: true,
                ..ComposeUpRequest::default()
            })
            .expect("compose up should succeed");

        assert_eq!(
            runner.invocations(),
            vec![CommandInvocation {
                program: OsString::from("docker"),
                args: vec![
                    OsString::from("compose"),
                    OsString::from("-f"),
                    OsString::from("compose.yml"),
                    OsString::from("-f"),
                    OsString::from("compose.override.yml"),
                    OsString::from("--project-directory"),
                    OsString::from("/workspace"),
                    OsString::from("--project-name"),
                    OsString::from("deco"),
                    OsString::from("up"),
                    OsString::from("-d"),
                    OsString::from("--build"),
                    OsString::from("--force-recreate"),
                    OsString::from("--remove-orphans"),
                    OsString::from("--wait"),
                    OsString::from("app"),
                    OsString::from("db"),
                ],
            }]
        );
    }

    #[test]
    fn compose_exec_formats_expected_arguments() {
        let runner = std::rc::Rc::new(FakeRunner::default());
        runner.push_response(Ok(CommandOutput {
            status: 0,
            stdout: "execed".to_string(),
            stderr: String::new(),
        }));

        let engine = DockerEngine::with_binary_and_runner("docker", runner.clone());
        engine
            .compose_exec(ComposeExecRequest {
                project: ComposeProjectRequest {
                    files: vec![PathBuf::from("compose.yml")],
                    project_name: Some("deco".to_string()),
                    ..ComposeProjectRequest::default()
                },
                service: "app".to_string(),
                command: vec!["sh".to_string(), "-lc".to_string(), "echo hi".to_string()],
                env: vec![("A".to_string(), "B".to_string())],
                workdir: Some("/workspace".to_string()),
                user: Some("1000:1000".to_string()),
                index: Some(2),
                tty: false,
                detach: true,
                privileged: true,
            })
            .expect("compose exec should succeed");

        assert_eq!(
            runner.invocations(),
            vec![CommandInvocation {
                program: OsString::from("docker"),
                args: vec![
                    OsString::from("compose"),
                    OsString::from("-f"),
                    OsString::from("compose.yml"),
                    OsString::from("--project-name"),
                    OsString::from("deco"),
                    OsString::from("exec"),
                    OsString::from("-d"),
                    OsString::from("-T"),
                    OsString::from("--index"),
                    OsString::from("2"),
                    OsString::from("--privileged"),
                    OsString::from("--workdir"),
                    OsString::from("/workspace"),
                    OsString::from("--user"),
                    OsString::from("1000:1000"),
                    OsString::from("--env"),
                    OsString::from("A=B"),
                    OsString::from("app"),
                    OsString::from("sh"),
                    OsString::from("-lc"),
                    OsString::from("echo hi"),
                ],
            }]
        );
    }

    #[test]
    fn compose_ps_formats_expected_arguments() {
        let runner = std::rc::Rc::new(FakeRunner::default());
        runner.push_response(Ok(CommandOutput {
            status: 0,
            stdout: r#"[{"ID":"abc","Service":"app","Name":"deco-app-1","State":"running"}]"#
                .to_string(),
            stderr: String::new(),
        }));

        let engine = DockerEngine::with_binary_and_runner("docker", runner.clone());
        let result = engine
            .compose_ps(ComposePsRequest {
                project: ComposeProjectRequest {
                    files: vec![PathBuf::from("compose.yml")],
                    project_name: Some("deco".to_string()),
                    ..ComposeProjectRequest::default()
                },
                services: vec!["app".to_string()],
                all: true,
                quiet: true,
                format_json: true,
            })
            .expect("compose ps should succeed");

        assert_eq!(result.raw[0]["ID"], "abc");
        assert_eq!(
            runner.invocations(),
            vec![CommandInvocation {
                program: OsString::from("docker"),
                args: vec![
                    OsString::from("compose"),
                    OsString::from("-f"),
                    OsString::from("compose.yml"),
                    OsString::from("--project-name"),
                    OsString::from("deco"),
                    OsString::from("ps"),
                    OsString::from("-a"),
                    OsString::from("-q"),
                    OsString::from("--format"),
                    OsString::from("json"),
                    OsString::from("app"),
                ],
            }]
        );
    }

    #[test]
    fn resolve_compose_target_prefers_running_entry() {
        let runner = std::rc::Rc::new(FakeRunner::default());
        runner.push_response(Ok(CommandOutput {
            status: 0,
            stdout: serde_json::json!([
                {"ID":"old","Service":"app","Name":"deco-app-1","State":"exited","Project":"deco"},
                {"ID":"new","Service":"app","Name":"deco-app-2","State":"running","Project":"deco"}
            ])
            .to_string(),
            stderr: String::new(),
        }));

        let engine = DockerEngine::with_binary_and_runner("docker", runner.clone());
        let result = engine
            .resolve_compose_target(ComposeTargetResolutionRequest {
                project: ComposeProjectRequest {
                    project_name: Some("deco".to_string()),
                    ..ComposeProjectRequest::default()
                },
                service: "app".to_string(),
                prefer_running: true,
            })
            .expect("compose target should resolve");

        assert_eq!(result.container_id.as_deref(), Some("new"));
        assert_eq!(result.container_name.as_deref(), Some("deco-app-2"));
        assert_eq!(
            runner.invocations(),
            vec![CommandInvocation {
                program: OsString::from("docker"),
                args: vec![
                    OsString::from("compose"),
                    OsString::from("--project-name"),
                    OsString::from("deco"),
                    OsString::from("ps"),
                    OsString::from("-a"),
                    OsString::from("--format"),
                    OsString::from("json"),
                    OsString::from("app"),
                ],
            }]
        );
    }

    #[test]
    fn create_formats_bind_mounts_before_image() {
        let runner = std::rc::Rc::new(FakeRunner::default());
        runner.push_response(Ok(CommandOutput {
            status: 0,
            stdout: "created".to_string(),
            stderr: String::new(),
        }));

        let engine = DockerEngine::with_binary_and_runner("docker", runner.clone());
        engine
            .create(ContainerCreateRequest {
                image: "alpine:3.20".to_string(),
                mounts: vec![
                    ContainerBindMount::new("/src", "/workspace/src"),
                    ContainerBindMount::new("/cache", "/workspace/cache").readonly(true),
                ],
                workdir: Some("/workspace".to_string()),
                ..ContainerCreateRequest::default()
            })
            .expect("create result");

        assert_eq!(
            runner.invocations(),
            vec![CommandInvocation {
                program: OsString::from("docker"),
                args: vec![
                    OsString::from("create"),
                    OsString::from("--workdir"),
                    OsString::from("/workspace"),
                    OsString::from("--mount"),
                    OsString::from("type=bind,source=/src,target=/workspace/src"),
                    OsString::from("--mount"),
                    OsString::from("type=bind,source=/cache,target=/workspace/cache,readonly"),
                    OsString::from("alpine:3.20"),
                ],
            }]
        );
    }
}
