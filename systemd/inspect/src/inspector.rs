use crate::data::{
    Error, KernelIdentity, MAX_ENTRIES, MAX_OUTPUT_BYTES, Membership, Schema, ServiceIdentity,
    Snapshot, StoreContents, reply, single, string, uint32,
};
use handle_off_systemd_fdstore::{
    Capabilities, Capacity, ManagerReport, RemoveReceipt, StoreReceipt, Support, Verification,
};
use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
const DESTINATION: &str = "org.freedesktop.systemd1";
const MANAGER_PATH: &str = "/org/freedesktop/systemd1";
const MANAGER: &str = "org.freedesktop.systemd1.Manager";
const UNIT: &str = "org.freedesktop.systemd1.Unit";
const SERVICE: &str = "org.freedesktop.systemd1.Service";
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BusScope {
    User,
    System,
}
#[derive(Clone, Debug)]
pub struct Target {
    unit: String,
    expected_main_pid: u32,
}
impl Target {
    /// Use the canonical service name, not an alias. Explicit PID may be this process
    /// for self-custody, or a known service MainPID for read-only external inspection.
    pub fn new(unit: impl Into<String>, expected_main_pid: u32) -> Result<Self, Error> {
        let unit = unit.into();
        if !unit.ends_with(".service")
            || unit.len() > 255
            || expected_main_pid == 0
            || !unit
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"_-.@:\\".contains(&b))
        {
            return Err(Error::InvalidConfiguration(
                "canonical service name and nonzero expected MainPID required",
            ));
        }
        Ok(Self {
            unit,
            expected_main_pid,
        })
    }
}
#[derive(Clone, Copy, Debug)]
pub struct QueryLimits {
    pub timeout: Duration,
    pub max_output_bytes: usize,
    pub max_entries: usize,
}
impl Default for QueryLimits {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(3),
            max_output_bytes: 1024 * 1024,
            max_entries: 16_384,
        }
    }
}
#[derive(Clone, Debug)]
pub struct Inspector {
    scope: BusScope,
    target: Target,
    limits: QueryLimits,
    program: PathBuf,
}
impl Inspector {
    pub fn new(scope: BusScope, target: Target, limits: QueryLimits) -> Result<Self, Error> {
        if limits.timeout.is_zero()
            || Instant::now().checked_add(limits.timeout).is_none()
            || rustix::event::Timespec::try_from(limits.timeout).is_err()
            || limits.max_output_bytes == 0
            || limits.max_output_bytes > MAX_OUTPUT_BYTES
            || limits.max_entries > MAX_ENTRIES
        {
            return Err(Error::InvalidConfiguration(
                "invalid query time/output/entry limits",
            ));
        }
        Ok(Self {
            scope,
            target,
            limits,
            program: PathBuf::from("busctl"),
        })
    }
    /// Trusted executable override, useful for installations and fixture tests.
    /// No shell interpolation. The executable must not daemonize/escape its group.
    pub fn with_program(mut self, program: impl AsRef<Path>) -> Self {
        self.program = program.as_ref().to_owned();
        self
    }
    fn query(&self, args: &[&str], deadline: Instant) -> Result<Vec<u8>, Error> {
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .filter(|d| !d.is_zero())
            .ok_or(Error::Timeout)?;
        let mut arguments: Vec<String> = [
            match self.scope {
                BusScope::User => "--user",
                BusScope::System => "--system",
            },
            "--json=short",
            "--no-pager",
            "--auto-start=no",
            "--allow-interactive-authorization=no",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect();
        arguments.push(format!(
            "--timeout={:.6}",
            remaining.as_secs_f64().max(0.000001)
        ));
        arguments.push("--".into());
        arguments.extend(args.iter().map(|arg| (*arg).to_owned()));
        crate::runner::run(
            &self.program,
            &arguments,
            deadline,
            self.limits.max_output_bytes,
        )
    }
    fn path(&self, deadline: Instant) -> Result<String, Error> {
        let bytes = self.query(
            &[
                "call",
                DESTINATION,
                MANAGER_PATH,
                MANAGER,
                "GetUnit",
                "s",
                &self.target.unit,
            ],
            deadline,
        )?;
        let value = reply(&bytes, "o")?;
        let path = string(single(&value)?)?;
        if !path.starts_with("/org/freedesktop/systemd1/unit/")
            || path
                .bytes()
                .any(|b| !b.is_ascii_alphanumeric() && b != b'/' && b != b'_')
        {
            return Err(Error::InvalidReply("invalid service object path"));
        }
        Ok(path)
    }
    fn schema(&self, path: &str, deadline: Instant) -> Result<Schema, Error> {
        let bytes = self.query(
            &[
                "call",
                DESTINATION,
                path,
                "org.freedesktop.DBus.Introspectable",
                "Introspect",
            ],
            deadline,
        )?;
        let value = reply(&bytes, "s")?;
        Schema::parse(
            single(&value)?
                .as_str()
                .ok_or(Error::InvalidReply("expected introspection XML string"))?,
        )
    }
    fn property(
        &self,
        path: &str,
        interface: &str,
        name: &str,
        signature: &str,
        deadline: Instant,
    ) -> Result<serde_json::Value, Error> {
        let bytes = self.query(
            &["get-property", DESTINATION, path, interface, name],
            deadline,
        )?;
        reply(&bytes, signature)
    }
    fn identity(
        &self,
        path: &str,
        schema: &Schema,
        deadline: Instant,
    ) -> Result<ServiceIdentity, Error> {
        for (interface, name, signature) in [
            (UNIT, "Id", "s"),
            (UNIT, "InvocationID", "ay"),
            (SERVICE, "MainPID", "u"),
        ] {
            if !schema.property(interface, name, signature)? {
                return Err(Error::Unsupported("service identity property unavailable"));
            }
        }
        let unit = string(&self.property(path, UNIT, "Id", "s", deadline)?)?;
        if unit != self.target.unit {
            return Err(Error::WrongUnit {
                expected: self.target.unit.clone(),
                actual: unit,
            });
        }
        let main_pid = uint32(&self.property(path, SERVICE, "MainPID", "u", deadline)?)?;
        if main_pid != self.target.expected_main_pid {
            return Err(Error::OwnerMismatch {
                expected: self.target.expected_main_pid,
                actual: main_pid,
            });
        }
        let value = self.property(path, UNIT, "InvocationID", "ay", deadline)?;
        let values = value
            .as_array()
            .ok_or(Error::InvalidReply("expected InvocationID byte array"))?;
        if values.len() != 16 {
            return Err(Error::InvalidReply("InvocationID must have 16 bytes"));
        }
        let mut invocation_id = [0; 16];
        for (slot, value) in invocation_id.iter_mut().zip(values) {
            *slot = u8::try_from(uint32(value)?)
                .map_err(|_| Error::InvalidReply("invalid InvocationID byte"))?;
        }
        if invocation_id == [0; 16] {
            return Err(Error::InvalidReply(
                "inactive/unknown InvocationID cannot establish service incarnation",
            ));
        }
        Ok(ServiceIdentity {
            unit,
            object_path: path.into(),
            main_pid,
            invocation_id,
        })
    }
    /// Aggregate deadline applies to all serial commands. Properties are sampled,
    /// not reserved; the service incarnation must remain stable across sampling.
    /// Dump support comes from introspection, not English error matching/version alone.
    pub fn inspect(&self) -> Result<Snapshot, Error> {
        let deadline = Instant::now()
            .checked_add(self.limits.timeout)
            .ok_or(Error::InvalidConfiguration("query deadline overflow"))?;
        let path = self.path(deadline)?;
        let schema = self.schema(&path, deadline)?;
        let identity = self.identity(&path, &schema, deadline)?;
        let manager_version =
            string(&self.property(MANAGER_PATH, MANAGER, "Version", "s", deadline)?)?;
        let numeric_version = manager_version
            .split(|c: char| !c.is_ascii_digit())
            .next()
            .filter(|v| !v.is_empty())
            .and_then(|v| v.parse::<u32>().ok());
        let support = |since| match numeric_version {
            Some(version) if version >= since => Support::Supported,
            Some(_) => Support::Unsupported,
            None => Support::Unknown,
        };
        let read_capacity = |name| -> Result<Option<u64>, Error> {
            if !schema.property(SERVICE, name, "u")? {
                return Ok(None);
            }
            Ok(Some(
                uint32(&self.property(&path, SERVICE, name, "u", deadline)?)? as u64,
            ))
        };
        let maximum = read_capacity("FileDescriptorStoreMax")?;
        let occupied = read_capacity("NFileDescriptorStore")?;
        let capacity = maximum
            .zip(occupied)
            .map(|(maximum, occupied)| Capacity { maximum, occupied });
        let contents = if !schema.dump {
            StoreContents::Unsupported
        } else if maximum == Some(0) && occupied == Some(0) {
            StoreContents::Disabled
        } else {
            match self.query(
                &[
                    "call",
                    DESTINATION,
                    &path,
                    SERVICE,
                    "DumpFileDescriptorStore",
                ],
                deadline,
            ) {
                Ok(bytes) => {
                    let entries = crate::data::parse_dump(&bytes, self.limits.max_entries)?;
                    if occupied.is_some_and(|count| count != entries.len() as u64) {
                        return Err(Error::InconsistentInventory);
                    }
                    StoreContents::Available(entries)
                }
                Err(Error::StoreDisabled) => StoreContents::Disabled,
                Err(Error::Unsupported(_)) => StoreContents::Unsupported,
                Err(error) => return Err(error),
            }
        };
        let after_path = self.path(deadline)?;
        if after_path != path || self.identity(&after_path, &schema, deadline)? != identity {
            return Err(Error::IdentityChanged);
        }
        let inspection = if matches!(contents, StoreContents::Unsupported) {
            Support::Unsupported
        } else {
            Support::Supported
        };
        Ok(Snapshot {
            identity,
            manager_version,
            report: ManagerReport {
                capacity,
                capabilities: Capabilities {
                    store: support(219),
                    remove: support(236),
                    poll_control: support(246),
                    barrier: support(246),
                    inspection,
                },
            },
            contents,
        })
    }
    pub fn verify_offer(&self, receipt: &StoreReceipt<'_>) -> Result<Verification, Error> {
        receipt.verify_with(|receipt| {
            let originals = receipt
                .descriptors
                .iter()
                .map(|fd| KernelIdentity::from_fd(*fd))
                .collect::<Result<Vec<_>, _>>()?;
            let snapshot = self.inspect()?;
            Ok(match snapshot.membership(&receipt.name, &originals) {
                Membership::MatchedMetadata { .. } => Verification::Confirmed,
                Membership::Unsupported => Verification::Unsupported,
                _ => Verification::Rejected,
            })
        })
    }
    /// Confirms observed absence, not causality or retirement of unnamed aliases.
    /// Does not perform a remove operation or modify any other manager state.
    pub fn verify_removal(&self, receipt: &RemoveReceipt) -> Result<Verification, Error> {
        receipt.verify_with(|receipt| {
            let snapshot = self.inspect()?;
            Ok(match snapshot.contents {
                StoreContents::Unsupported => Verification::Unsupported,
                StoreContents::Disabled => Verification::Confirmed,
                StoreContents::Available(entries)
                    if entries.iter().any(|entry| entry.name == receipt.name) =>
                {
                    Verification::Rejected
                }
                StoreContents::Available(_) => Verification::Confirmed,
            })
        })
    }
}
