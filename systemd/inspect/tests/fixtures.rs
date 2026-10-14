#![cfg(target_os = "linux")]
use handle_off_systemd_fdstore::{FdName, StoreReceipt, Verification};
use handle_off_systemd_inspect::{
    BusScope, Error, Inspector, KernelIdentity, QueryLimits, StoreContents, Target,
};
use std::{
    os::{
        fd::AsFd,
        unix::{
            fs::{DirBuilderExt, PermissionsExt},
            net::UnixStream,
        },
    },
    path::PathBuf,
    time::{Duration, Instant},
};
struct Fixture {
    directory: PathBuf,
    program: PathBuf,
}
impl Fixture {
    fn new(name: &str, mode: &str, identity: &KernelIdentity) -> Self {
        let root =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.test-agent/systemd-inspect");
        std::fs::create_dir_all(&root).unwrap();
        let readme = root.join("README.md");
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(readme)
        {
            use std::io::Write;
            file.write_all(b"# Inspection fixtures\n\nCargo tests generate trusted local Python fake-busctl scripts here. Each test owns a unique process-specific directory and removes its own files. No live manager state is changed.\n").unwrap();
        }
        let directory = root.join(format!("{}-{name}", std::process::id()));
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(&directory)
            .unwrap();
        let program = directory.join("busctl.py");
        let row = serde_json::json!([
            "socket",
            identity.mode,
            identity.device_major,
            identity.device_minor,
            identity.inode,
            identity.rdev_major,
            identity.rdev_minor,
            "socket",
            2
        ]);
        let config = serde_json::json!({"mode":mode,"owner":std::process::id(),"row":row,"directory":directory});
        let source = format!(
            "#!/usr/bin/env python3\nimport json, os, sys, time, subprocess\nconfig = json.loads(r'''{config}''')\n"
        ) + r#"
mode = config['mode']
assert '--json=short' in sys.argv and '--auto-start=no' in sys.argv
assert '--allow-interactive-authorization=no' in sys.argv and '--no-pager' in sys.argv
assert ('--system' if mode == 'system' else '--user') in sys.argv
if mode == 'permission':
    print('Call failed: Access denied', file=sys.stderr); sys.exit(1)
if mode == 'command':
    print('fixture command failed', file=sys.stderr); sys.exit(3)
if mode in ('timeout', 'output'):
    child = subprocess.Popen([sys.executable, '-c', 'import time; time.sleep(60)'])
    open(config['directory'] + '/pids', 'w').write(str(os.getpid()) + ' ' + str(child.pid))
    if mode == 'output':
        while True: sys.stdout.write('x' * 8192); sys.stdout.flush()
    time.sleep(60)
args = sys.argv[sys.argv.index('--') + 1:]
def emit(kind, data): print(json.dumps({'type':kind,'data':data}))
if args[0] == 'call':
    method = args[4]
    if method == 'GetUnit': emit('o', ['/org/freedesktop/systemd1/unit/demo_2eservice'])
    elif method == 'Introspect':
        maximum = '' if mode == 'capacity-unknown' else '<property name="FileDescriptorStoreMax" type="u" access="read"/>'
        dump = '' if mode == 'unsupported' else '<method name="DumpFileDescriptorStore"><arg direction="out" type="a(suuutuusu)"/></method>'
        emit('s', ['<!DOCTYPE node PUBLIC "-//freedesktop//DTD D-BUS Object Introspection 1.0//EN" "https://www.freedesktop.org/standards/dbus/1.0/introspect.dtd"><node><interface name="org.freedesktop.systemd1.Unit"><property name="Id" type="s" access="read"/><property name="InvocationID" type="ay" access="read"/></interface><interface name="org.freedesktop.systemd1.Service"><property name="MainPID" type="u" access="read"/><property name="NFileDescriptorStore" type="u" access="read"/>' + maximum + dump + '</interface></node>'])
    elif method == 'DumpFileDescriptorStore':
        rows = [config['row']]
        if mode == 'missing': rows = []
        if mode == 'wrong-fd': rows[0][4] += 1
        if mode == 'duplicate': rows.append(list(rows[0]))
        if mode == 'unexpected':
            extra = list(rows[0]); extra[4] += 1; rows.append(extra)
        if mode == 'malformed': print('{malformed')
        else: emit('a(suuutuusu)', [rows])
    else: raise RuntimeError('unknown method')
else:
    name = args[-1]
    if name == 'Version': emit('s', '262-1')
    elif name == 'Id': emit('s', 'wrong.service' if mode == 'wrong-unit' else 'demo.service')
    elif name == 'MainPID': emit('u', config['owner'] + (1 if mode == 'wrong-pid' else 0))
    elif name == 'InvocationID':
        countfile = config['directory'] + '/invocations'
        count = int(open(countfile).read()) if os.path.exists(countfile) else 0
        open(countfile, 'w').write(str(count + 1))
        emit('ay', [2 if mode == 'changed' and count > 0 else 1] * 16)
    elif name == 'FileDescriptorStoreMax': emit('u', 0 if mode == 'disabled' else 2048)
    elif name == 'NFileDescriptorStore': emit('u', 0 if mode in ('missing', 'disabled') else 2 if mode in ('duplicate', 'unexpected', 'inconsistent') else 1)
    else: raise RuntimeError('unknown property')
"#;
        std::fs::write(&program, source).unwrap();
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700)).unwrap();
        Self { directory, program }
    }
    fn inspector(&self, limits: QueryLimits) -> Inspector {
        Inspector::new(
            BusScope::User,
            Target::new("demo.service", std::process::id()).unwrap(),
            limits,
        )
        .unwrap()
        .with_program(&self.program)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        for name in ["busctl.py", "invocations", "pids"] {
            match std::fs::remove_file(self.directory.join(name)) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => panic!("fixture cleanup: {error}"),
            }
        }
        std::fs::remove_dir(&self.directory).unwrap();
    }
}
#[test]
fn bounded_fixture_queries_verify_actual_borrowed_originals_not_name_alone() {
    let (socket, _peer) = UnixStream::pair().unwrap();
    let identity = KernelIdentity::from_fd(socket.as_fd()).unwrap();
    let receipt = StoreReceipt {
        name: FdName::new("socket").unwrap(),
        descriptors: vec![socket.as_fd()],
        submission: handle_off_systemd_notify::Submission::Submitted,
    };
    for (mode, expected) in [
        ("ok", Verification::Confirmed),
        ("wrong-fd", Verification::Rejected),
        ("missing", Verification::Rejected),
        ("duplicate", Verification::Rejected),
        ("unexpected", Verification::Rejected),
        ("unsupported", Verification::Unsupported),
        ("disabled", Verification::Rejected),
        ("capacity-unknown", Verification::Confirmed),
    ] {
        let fixture = Fixture::new(mode, mode, &identity);
        let inspector = fixture.inspector(QueryLimits::default());
        assert_eq!(
            inspector.verify_offer(&receipt).unwrap(),
            expected,
            "mode {mode}"
        );
        if mode == "capacity-unknown" {
            assert!(inspector.inspect().unwrap().report.capacity.is_none());
        }
    }
}
#[test]
fn permission_command_schema_and_identity_errors_are_not_absent_storage() {
    let (socket, _peer) = UnixStream::pair().unwrap();
    let identity = KernelIdentity::from_fd(socket.as_fd()).unwrap();
    for mode in [
        "permission",
        "command",
        "wrong-unit",
        "wrong-pid",
        "changed",
        "malformed",
        "inconsistent",
    ] {
        let fixture = Fixture::new(mode, mode, &identity);
        let error = fixture
            .inspector(QueryLimits::default())
            .inspect()
            .unwrap_err();
        assert!(match (mode, error) {
            ("permission", Error::PermissionDenied(_))
            | ("command", Error::CommandFailed { .. })
            | ("wrong-unit", Error::WrongUnit { .. })
            | ("wrong-pid", Error::OwnerMismatch { .. })
            | ("changed", Error::IdentityChanged)
            | ("malformed", Error::Json(_))
            | ("inconsistent", Error::InconsistentInventory) => true,
            (mode, error) => panic!("{mode}: {error}"),
        });
    }
}
fn running(pid: u32) -> bool {
    std::fs::read_to_string(format!("/proc/{pid}/stat"))
        .ok()
        .and_then(|stat| {
            stat.rsplit_once(") ")
                .map(|(_, rest)| rest.starts_with('Z') || rest.starts_with('X'))
        })
        .is_some_and(|dead| !dead)
}
#[test]
fn deadline_and_output_limit_kill_query_group_and_reap_direct_child() {
    let (socket, _peer) = UnixStream::pair().unwrap();
    let identity = KernelIdentity::from_fd(socket.as_fd()).unwrap();
    for mode in ["timeout", "output"] {
        let fixture = Fixture::new(mode, mode, &identity);
        let limits = QueryLimits {
            timeout: Duration::from_millis(500),
            max_output_bytes: 1024,
            ..QueryLimits::default()
        };
        let error = fixture.inspector(limits).inspect().unwrap_err();
        assert!(matches!(
            (mode, error),
            ("timeout", Error::Timeout) | ("output", Error::OutputLimit)
        ));
        let pids: Vec<u32> = std::fs::read_to_string(fixture.directory.join("pids"))
            .unwrap()
            .split_whitespace()
            .map(|pid| pid.parse().unwrap())
            .collect();
        assert!(
            !PathBuf::from(format!("/proc/{}", pids[0])).exists(),
            "direct child must be reaped"
        );
        let deadline = Instant::now() + Duration::from_secs(1);
        while running(pids[1]) && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(2));
        }
        assert!(
            !running(pids[1]),
            "same-group descendant must not remain running"
        );
    }
}
#[test]
fn no_manager_bypasses_external_queries_and_configuration_is_checked() {
    let (socket, _peer) = UnixStream::pair().unwrap();
    let receipt = StoreReceipt {
        name: FdName::new("socket").unwrap(),
        descriptors: vec![socket.as_fd()],
        submission: handle_off_systemd_notify::Submission::NoManager,
    };
    let inspector = Inspector::new(
        BusScope::System,
        Target::new("demo.service", std::process::id()).unwrap(),
        QueryLimits::default(),
    )
    .unwrap()
    .with_program("/nonexistent/busctl");
    assert_eq!(
        inspector.verify_offer(&receipt).unwrap(),
        Verification::NoManager
    );
    assert!(Target::new("invalid/path.service", 42).is_err());
    assert!(Target::new("demo.service", 0).is_err());
    assert!(
        Inspector::new(
            BusScope::User,
            Target::new("demo.service", 42).unwrap(),
            QueryLimits {
                timeout: Duration::ZERO,
                ..QueryLimits::default()
            }
        )
        .is_err()
    );
}

#[test]
fn system_scope_and_removal_verify_observed_absence_without_mutation() {
    use handle_off_systemd_fdstore::RemoveReceipt;
    let (socket, _peer) = UnixStream::pair().unwrap();
    let identity = KernelIdentity::from_fd(socket.as_fd()).unwrap();
    let removal = RemoveReceipt {
        name: FdName::new("socket").unwrap(),
        submission: handle_off_systemd_notify::Submission::Submitted,
    };
    for (mode, expected) in [
        ("missing", Verification::Confirmed),
        ("ok", Verification::Rejected),
        ("unsupported", Verification::Unsupported),
    ] {
        let fixture = Fixture::new(&format!("removal-{mode}"), mode, &identity);
        assert_eq!(
            fixture
                .inspector(QueryLimits::default())
                .verify_removal(&removal)
                .unwrap(),
            expected
        );
    }
    let fixture = Fixture::new("system-scope", "system", &identity);
    let inspector = Inspector::new(
        BusScope::System,
        Target::new("demo.service", std::process::id()).unwrap(),
        QueryLimits::default(),
    )
    .unwrap()
    .with_program(&fixture.program);
    assert_eq!(
        inspector.inspect().unwrap().identity.main_pid,
        std::process::id()
    );
}
#[test]
#[ignore = "optional read-only user-manager check; requires running canonical service and explicit expected PID"]
fn live_read_only_service_snapshot() {
    let unit = std::env::var("HANDLE_OFF_INSPECT_UNIT").expect("canonical service name required");
    let pid = std::env::var("HANDLE_OFF_INSPECT_PID")
        .expect("explicit MainPID required")
        .parse()
        .unwrap();
    let inspector = Inspector::new(
        BusScope::User,
        Target::new(unit, pid).unwrap(),
        QueryLimits::default(),
    )
    .unwrap();
    let snapshot = inspector.inspect().unwrap();
    assert_eq!(snapshot.identity.main_pid, pid);
    assert!(!matches!(snapshot.contents, StoreContents::Unsupported));
}
