use handle_off_rendezvous::{
    Candidate, Endpoint, Environment, InvalidName, Origin, ResolveError, Resolver, ServiceName,
};
use std::{
    ffi::{OsStr, OsString},
    fmt,
    path::{Path, PathBuf},
};

/// The conventional variable this policy consults; a fixed open standard,
/// not application branding. Optional in every sense: unset or empty simply
/// disables the source.
pub const XDG_RUNTIME_DIR_VAR: &str = "XDG_RUNTIME_DIR";

/// Socket suffix of derived flat filename candidates.
pub const SOCKET_SUFFIX: &str = ".sock";

/// Default consultation order: explicit, environment, XDG runtime directory,
/// local directory. XDG precedes the local directory because a per-user
/// runtime directory is the better-scoped convention; reorder explicitly
/// when a different precedence is wanted.
pub const DEFAULT_ORDER: [Source; 4] = [
    Source::Explicit,
    Source::Environment,
    Source::XdgRuntimeDir,
    Source::LocalDir,
];

const ORIGIN_EXPLICIT: &str = "explicit";
const ORIGIN_ENVIRONMENT: &str = "environment";
const ORIGIN_XDG: &str = "xdg-runtime-dir";
const ORIGIN_LOCAL: &str = "local-dir";

/// One rendezvous source this policy can consult.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Source {
    /// A typed [`Endpoint`] configured with [`LocalPolicy::with_endpoint`].
    Explicit,
    /// An environment variable, named by the caller, whose value follows the
    /// [`Endpoint::parse_value`] text convention.
    Environment,
    /// `XDG_RUNTIME_DIR` from the environment snapshot.
    XdgRuntimeDir,
    /// A caller-supplied base directory.
    LocalDir,
}

/// A validated policy namespace, governed by the same rules as
/// [`ServiceName`]; embedded into flat candidate filenames, so it can never
/// introduce a separator or `..` component.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Namespace(ServiceName);
impl Namespace {
    pub fn new(value: impl Into<String>) -> Result<Self, InvalidName> {
        Ok(Self(ServiceName::new(value)?))
    }
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}
impl fmt::Display for Namespace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The sample explicit/environment/XDG/local policy.
///
/// ```
/// use handle_off_rendezvous::Environment;
/// use handle_off_rendezvous_local::LocalPolicy;
/// use std::ffi::OsString;
///
/// let environment = Environment::from_pairs([(
///     OsString::from("XDG_RUNTIME_DIR"),
///     OsString::from("/run/user/1000"),
/// )]);
/// let policy = LocalPolicy::new("demo", environment).unwrap();
/// let found = policy.candidates_for("ctl").unwrap();
/// assert_eq!(found.len(), 1);
/// assert_eq!(
///     found[0].endpoint.as_path().unwrap(),
///     std::path::Path::new("/run/user/1000/demo-ctl.sock")
/// );
/// assert_eq!(found[0].origin.as_str(), "xdg-runtime-dir");
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalPolicy {
    namespace: Namespace,
    environment: Environment,
    endpoint: Option<Endpoint>,
    endpoint_variable: Option<OsString>,
    local_directory: Option<PathBuf>,
    order: Vec<Source>,
}

impl LocalPolicy {
    /// The namespace participates in every derived filename; the environment
    /// snapshot is consumed read-only. Both the namespace and, later, each
    /// service name are validated: a bad namespace fails here, a bad name
    /// fails per resolution.
    pub fn new(
        namespace: impl Into<String>,
        environment: Environment,
    ) -> Result<Self, InvalidName> {
        Ok(Self {
            namespace: Namespace::new(namespace)?,
            environment,
            endpoint: None,
            endpoint_variable: None,
            local_directory: None,
            order: DEFAULT_ORDER.to_vec(),
        })
    }

    /// Configures the explicit endpoint. Typed endpoints are valid by
    /// construction; invalid explicit *text* should fail at
    /// [`Endpoint::parse_value`] before reaching the policy.
    pub fn with_endpoint(mut self, endpoint: Endpoint) -> Self {
        self.endpoint = Some(endpoint);
        self
    }

    /// Names the environment variable the [`Source::Environment`] source
    /// consults in the snapshot. Caller-chosen; nothing is presumed.
    pub fn with_endpoint_variable(mut self, name: impl Into<OsString>) -> Self {
        self.endpoint_variable = Some(name.into());
        self
    }

    /// Configures the base directory for [`Source::LocalDir`] candidates.
    /// Not created, verified, or canonicalized by this policy.
    pub fn with_local_directory(mut self, directory: impl Into<PathBuf>) -> Self {
        self.local_directory = Some(directory.into());
        self
    }

    /// Sets the consultation order verbatim; it is used exactly as given, so
    /// an empty order disables every source and duplicated sources yield at
    /// their first occurrence. Sources after the first that yields are not
    /// consulted.
    pub fn with_order(mut self, order: impl Into<Vec<Source>>) -> Self {
        self.order = order.into();
        self
    }

    pub fn namespace(&self) -> &str {
        self.namespace.as_str()
    }

    pub fn environment(&self) -> &Environment {
        &self.environment
    }

    pub fn endpoint(&self) -> Option<&Endpoint> {
        self.endpoint.as_ref()
    }

    pub fn endpoint_variable(&self) -> Option<&OsStr> {
        self.endpoint_variable.as_deref()
    }

    pub fn local_directory(&self) -> Option<&Path> {
        self.local_directory.as_deref()
    }

    pub fn order(&self) -> &[Source] {
        &self.order
    }

    /// Ordered, unverified candidates for `name`. No source connects,
    /// launches, authenticates, or removes anything; see the crate docs for
    /// each source's yield/error/none semantics and the first-yield
    /// termination rule.
    pub fn candidates(&self, name: &ServiceName) -> Result<Vec<Candidate>, ResolveError> {
        for &source in &self.order {
            match source {
                Source::Explicit => {
                    if let Some(endpoint) = &self.endpoint {
                        return Ok(vec![Candidate::new(
                            endpoint.clone(),
                            Origin::new(ORIGIN_EXPLICIT),
                        )]);
                    }
                }
                Source::Environment => {
                    if let Some(variable) = &self.endpoint_variable
                        && let Some(value) = self.environment.get(variable)
                    {
                        let origin = Origin::new(format!(
                            "{ORIGIN_ENVIRONMENT}:{}",
                            variable.to_string_lossy()
                        ));
                        let endpoint = Endpoint::parse_value(value).map_err(|invalid| {
                            ResolveError::Endpoint {
                                origin: origin.clone(),
                                invalid,
                            }
                        })?;
                        return Ok(vec![Candidate::new(endpoint, origin)]);
                    }
                }
                Source::XdgRuntimeDir => {
                    if let Some(directory) = self.xdg_runtime_directory() {
                        return self.flat_path_candidate(directory, name, ORIGIN_XDG);
                    }
                }
                Source::LocalDir => {
                    if let Some(directory) = &self.local_directory {
                        return self.flat_path_candidate(directory, name, ORIGIN_LOCAL);
                    }
                }
            }
        }
        Ok(Vec::new())
    }

    /// Convenience wrapper that validates the name first, mapping a bad one
    /// to [`ResolveError::Name`] instead of requiring a prebuilt
    /// [`ServiceName`].
    pub fn candidates_for(&self, name: &str) -> Result<Vec<Candidate>, ResolveError> {
        let name = ServiceName::new(name).map_err(ResolveError::Name)?;
        self.candidates(&name)
    }

    fn xdg_runtime_directory(&self) -> Option<&OsStr> {
        match self.environment.get(XDG_RUNTIME_DIR_VAR) {
            Some(value) if !value.is_empty() => Some(value),
            _ => None,
        }
    }

    fn flat_path_candidate(
        &self,
        directory: impl AsRef<Path>,
        name: &ServiceName,
        origin: &str,
    ) -> Result<Vec<Candidate>, ResolveError> {
        let filename = format!("{}-{name}{SOCKET_SUFFIX}", self.namespace.as_str());
        let path = directory.as_ref().join(filename);
        let endpoint = Endpoint::path(path).map_err(|invalid| ResolveError::Endpoint {
            origin: Origin::new(origin),
            invalid,
        })?;
        Ok(vec![Candidate::new(endpoint, Origin::new(origin))])
    }
}
impl Resolver for LocalPolicy {
    fn candidates(&self, name: &ServiceName) -> Result<Vec<Candidate>, ResolveError> {
        self.candidates(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use handle_off_rendezvous::InvalidEndpoint;
    use std::os::unix::ffi::OsStringExt;

    fn snapshot(pairs: &[(OsString, OsString)]) -> Environment {
        Environment::from_pairs(pairs.iter().cloned())
    }

    fn xdg(value: &OsStr) -> (OsString, OsString) {
        (OsString::from(XDG_RUNTIME_DIR_VAR), value.to_os_string())
    }

    const VAR: &str = "DEMO_SOCKET";

    #[test]
    fn default_order_prefers_xdg_over_local_directory() {
        let environment = snapshot(&[xdg(OsStr::new("/run/user/1000"))]);
        let policy = LocalPolicy::new("app", environment)
            .unwrap()
            .with_local_directory("/tmp");
        let found = policy.candidates_for("ctl").unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(
            found[0].endpoint.as_path().unwrap(),
            Path::new("/run/user/1000/app-ctl.sock")
        );
        assert_eq!(found[0].origin.as_str(), ORIGIN_XDG);
    }

    #[test]
    fn order_controls_precedence_verbatim() {
        let environment = snapshot(&[xdg(OsStr::new("/run/user/1000"))]);
        let policy = LocalPolicy::new("app", environment)
            .unwrap()
            .with_local_directory("/tmp")
            .with_order([Source::LocalDir, Source::XdgRuntimeDir]);
        let found = policy.candidates_for("ctl").unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(
            found[0].endpoint.as_path().unwrap(),
            Path::new("/tmp/app-ctl.sock")
        );
        assert_eq!(found[0].origin.as_str(), ORIGIN_LOCAL);
    }

    #[test]
    fn explicit_endpoint_terminates_before_later_sources_are_consulted() {
        // The environment value is invalid; it must not even be parsed,
        // because the explicit endpoint ends resolution first.
        let environment = snapshot(&[
            xdg(OsStr::new("/run/user/1000")),
            (VAR.into(), OsString::from("@")),
        ]);
        let explicit = Endpoint::abstract_name(b"demo").unwrap();
        let policy = LocalPolicy::new("app", environment)
            .unwrap()
            .with_endpoint(explicit.clone())
            .with_endpoint_variable(VAR)
            .with_local_directory("/tmp");
        let found = policy.candidates_for("ctl").unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].endpoint, explicit);
        assert_eq!(found[0].origin.as_str(), ORIGIN_EXPLICIT);
    }

    #[test]
    fn environment_values_keep_both_forms_and_native_bytes() {
        for (value, expected) in [
            (
                OsString::from("/run/demo.sock"),
                Endpoint::path("/run/demo.sock").unwrap(),
            ),
            (
                OsString::from("@abs"),
                Endpoint::abstract_name(b"abs").unwrap(),
            ),
            (
                OsString::from_vec(b"/tmp/demo-\xff.sock".to_vec()),
                Endpoint::path(PathBuf::from(OsString::from_vec(
                    b"/tmp/demo-\xff.sock".to_vec(),
                )))
                .unwrap(),
            ),
        ] {
            let environment = snapshot(&[(VAR.into(), value)]);
            let policy = LocalPolicy::new("app", environment)
                .unwrap()
                .with_endpoint_variable(VAR);
            let found = policy.candidates_for("ctl").unwrap();
            assert_eq!(found.len(), 1, "one candidate for {expected}");
            assert_eq!(found[0].endpoint, expected);
            assert_eq!(
                found[0].origin.as_str(),
                format!("{ORIGIN_ENVIRONMENT}:{VAR}")
            );
        }
    }

    #[test]
    fn present_but_invalid_environment_value_fails_predictably() {
        for value in [OsString::from("@"), OsString::from("")] {
            let environment = snapshot(&[(VAR.into(), value.clone())]);
            let policy = LocalPolicy::new("app", environment)
                .unwrap()
                .with_endpoint_variable(VAR)
                .with_local_directory("/tmp");
            let error = policy.candidates_for("ctl").unwrap_err();
            assert_eq!(
                error,
                ResolveError::Endpoint {
                    origin: Origin::new(format!("{ORIGIN_ENVIRONMENT}:{VAR}")),
                    invalid: InvalidEndpoint::Empty,
                },
                "invalid value {value:?} must error, not fall through to /tmp"
            );
        }
    }

    #[test]
    fn absent_environment_variable_falls_through_to_next_source() {
        // Configured variable but the snapshot never carried it.
        let environment = snapshot(&[xdg(OsStr::new("/run/user/1000"))]);
        let policy = LocalPolicy::new("app", environment)
            .unwrap()
            .with_endpoint_variable(VAR);
        let found = policy.candidates_for("ctl").unwrap();
        assert_eq!(
            found[0].endpoint.as_path().unwrap(),
            Path::new("/run/user/1000/app-ctl.sock")
        );
    }

    #[test]
    fn empty_xdg_runtime_dir_behaves_as_unset() {
        let environment = snapshot(&[xdg(OsStr::new(""))]);
        let policy = LocalPolicy::new("app", environment)
            .unwrap()
            .with_local_directory("/tmp");
        let found = policy.candidates_for("ctl").unwrap();
        assert_eq!(
            found[0].endpoint.as_path().unwrap(),
            Path::new("/tmp/app-ctl.sock")
        );
        assert_eq!(found[0].origin.as_str(), ORIGIN_LOCAL);
    }

    #[test]
    fn nothing_configured_yields_no_candidates_rather_than_an_error() {
        let policy = LocalPolicy::new("app", Environment::default()).unwrap();
        assert_eq!(policy.candidates_for("ctl").unwrap(), Vec::new());
    }

    #[test]
    fn namespace_and_name_validations_govern_embedding() {
        assert_eq!(
            LocalPolicy::new("a/b", Environment::default()).unwrap_err(),
            InvalidName::Separator
        );
        let policy = LocalPolicy::new("ns", Environment::default()).unwrap();
        assert_eq!(
            policy.candidates_for("a/b").unwrap_err(),
            ResolveError::Name(InvalidName::Separator)
        );
        // Flat component: validated names cannot separate or traverse.
        let derived = policy
            .clone()
            .with_local_directory("/tmp")
            .candidates_for("..")
            .unwrap();
        assert_eq!(
            derived[0].endpoint.as_path().unwrap(),
            Path::new("/tmp/ns-...sock")
        );
    }

    #[test]
    fn interior_nul_directory_inputs_fail_predictably() {
        let with_nul = PathBuf::from(OsString::from_vec(b"/tmp/a\0b".to_vec()));
        let error = LocalPolicy::new("app", Environment::default())
            .unwrap()
            .with_local_directory(with_nul)
            .candidates_for("ctl")
            .unwrap_err();
        assert_eq!(
            error,
            ResolveError::Endpoint {
                origin: Origin::new(ORIGIN_LOCAL),
                invalid: InvalidEndpoint::InteriorNul,
            }
        );
        let environment = snapshot(&[xdg(&OsString::from_vec(b"/run/a\0b".to_vec()))]);
        let error = LocalPolicy::new("app", environment)
            .unwrap()
            .candidates_for("ctl")
            .unwrap_err();
        assert_eq!(
            error,
            ResolveError::Endpoint {
                origin: Origin::new(ORIGIN_XDG),
                invalid: InvalidEndpoint::InteriorNul,
            }
        );
    }

    #[test]
    fn variable_names_may_carry_native_bytes() {
        let variable = OsString::from_vec(b"DEMO_\xff".to_vec());
        let environment = snapshot(&[(variable.clone(), OsString::from("@abs"))]);
        let policy = LocalPolicy::new("app", environment)
            .unwrap()
            .with_endpoint_variable(variable);
        let found = policy.candidates_for("ctl").unwrap();
        assert_eq!(found[0].endpoint.as_abstract().unwrap().as_bytes(), b"abs");
        assert_eq!(
            found[0].origin.as_str(),
            format!("{ORIGIN_ENVIRONMENT}:DEMO_\u{fffd}")
        );
    }

    #[test]
    fn resolver_trait_object_composition() {
        let environment = snapshot(&[(VAR.into(), OsString::from("@trait"))]);
        let policy = LocalPolicy::new("app", environment)
            .unwrap()
            .with_endpoint_variable(VAR);
        let resolver: &dyn Resolver = &policy;
        let name = ServiceName::new("ctl").unwrap();
        let found = resolver.candidates(&name).unwrap();
        assert_eq!(
            found[0].endpoint.as_abstract().unwrap().as_bytes(),
            b"trait"
        );
    }
}
