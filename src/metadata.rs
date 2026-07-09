//! Workspace discovery: run `cargo metadata` once and turn it into a small,
//! tool-agnostic model — the members, their targets and declared dependencies,
//! and (optionally) the resolved dependency graph. The JSON parsing is split
//! out as [`parse`] so it can be tested without invoking cargo.

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::Error;

/// A discovered workspace.
pub struct Workspace {
    pub root: PathBuf,
    /// The real target directory (honours `CARGO_TARGET_DIR`).
    pub target_dir: PathBuf,
    pub members: Vec<Package>,
    /// member id → its direct workspace-member dependencies (dev edges dropped).
    /// `None` unless loaded with [`LoadOptions::resolve`].
    member_edges: Option<HashMap<String, Vec<String>>>,
}

/// A workspace member package.
pub struct Package {
    pub id: String,
    pub name: String,
    /// The directory holding the crate's `Cargo.toml`.
    pub manifest_dir: PathBuf,
    pub description: Option<String>,
    pub edition: String,
    pub license: Option<String>,
    pub targets: Vec<Target>,
    pub dependencies: Vec<Dependency>,
}

/// A build target of a package (a bin, the lib, …).
pub struct Target {
    pub name: String,
    pub kind: TargetKind,
    pub src_path: PathBuf,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TargetKind {
    Bin,
    Lib,
    Other,
}

/// A declared dependency.
pub struct Dependency {
    pub name: String,
    pub rename: Option<String>,
    pub req: String,
    pub features: Vec<String>,
    pub kind: DepKind,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DepKind {
    Normal,
    Dev,
    Build,
}

impl Workspace {
    pub fn member(&self, name: &str) -> Option<&Package> {
        self.members.iter().find(|p| p.name == name)
    }

    /// The transitive set of workspace members that `id` links (including
    /// itself), following non-dev member edges — the basis for per-crate
    /// staleness. `None` if the resolve graph wasn't requested.
    pub fn linked_members(&self, id: &str) -> Option<HashSet<String>> {
        let edges = self.member_edges.as_ref()?;
        let mut seen = HashSet::new();
        let mut queue = VecDeque::from([id.to_string()]);
        while let Some(cur) = queue.pop_front() {
            if !seen.insert(cur.clone()) {
                continue;
            }
            if let Some(neighbours) = edges.get(&cur) {
                queue.extend(neighbours.iter().cloned());
            }
        }
        Some(seen)
    }
}

impl Package {
    pub fn manifest_path(&self) -> PathBuf {
        self.manifest_dir.join("Cargo.toml")
    }

    pub fn bin_target(&self) -> Option<&Target> {
        self.targets.iter().find(|t| t.kind == TargetKind::Bin)
    }

    pub fn lib_target(&self) -> Option<&Target> {
        self.targets.iter().find(|t| t.kind == TargetKind::Lib)
    }

    /// The crate root file: the lib target's source, else the first bin's.
    pub fn crate_root(&self) -> Option<&Path> {
        self.lib_target()
            .or_else(|| self.bin_target())
            .map(|t| t.src_path.as_path())
    }

    /// The directory the crate root lives in (usually `src/`).
    pub fn src_dir(&self) -> Option<&Path> {
        self.crate_root().and_then(Path::parent)
    }

    pub fn has_dependency(&self, name: &str) -> bool {
        self.dependencies.iter().any(|d| d.name == name)
    }
}

impl Dependency {
    /// The identifier this dep is referred to by in source (rename wins, else
    /// the package name with dashes turned into underscores).
    pub fn extern_ident(&self) -> String {
        self.rename
            .clone()
            .unwrap_or_else(|| self.name.replace('-', "_"))
    }

    pub fn is_normal(&self) -> bool {
        self.kind == DepKind::Normal
    }
}

/// How to run discovery.
#[derive(Default)]
pub struct LoadOptions {
    /// Point cargo at a specific workspace instead of the cwd.
    pub manifest_path: Option<PathBuf>,
    /// Include the resolved dependency graph (needed for [`Workspace::linked_members`]).
    /// Slower, since it omits `--no-deps`.
    pub resolve: bool,
}

/// Run `cargo metadata` and build the [`Workspace`].
pub fn load(options: &LoadOptions) -> Result<Workspace, Error> {
    let mut cmd = Command::new("cargo");
    cmd.args(["metadata", "--format-version", "1"]);
    if !options.resolve {
        cmd.arg("--no-deps");
    }
    if let Some(mp) = &options.manifest_path {
        cmd.arg("--manifest-path").arg(mp);
    }
    let output = cmd.output().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            Error::CargoNotFound
        } else {
            Error::Metadata(e.to_string())
        }
    })?;
    if !output.status.success() {
        let msg = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(Error::NotWorkspace(msg));
    }
    parse(&output.stdout)
}

/// Parse `cargo metadata --format-version 1` JSON into a [`Workspace`].
pub fn parse(json: &[u8]) -> Result<Workspace, Error> {
    let meta: raw::Metadata =
        serde_json::from_slice(json).map_err(|e| Error::Parse(e.to_string()))?;
    let member_ids: HashSet<&str> = meta.workspace_members.iter().map(String::as_str).collect();

    let members = meta
        .packages
        .iter()
        .filter(|p| member_ids.contains(p.id.as_str()))
        .map(package_from_raw)
        .collect();

    let member_edges = meta.resolve.as_ref().map(|r| member_edges(r, &member_ids));

    Ok(Workspace {
        root: PathBuf::from(meta.workspace_root),
        target_dir: PathBuf::from(meta.target_directory),
        members,
        member_edges,
    })
}

fn package_from_raw(p: &raw::Package) -> Package {
    let manifest_dir = Path::new(&p.manifest_path)
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_default();
    Package {
        id: p.id.clone(),
        name: p.name.clone(),
        manifest_dir,
        description: p.description.clone(),
        edition: p.edition.clone().unwrap_or_else(|| "2015".to_string()),
        license: p.license.clone(),
        targets: p.targets.iter().map(target_from_raw).collect(),
        dependencies: p.dependencies.iter().map(dep_from_raw).collect(),
    }
}

fn target_from_raw(t: &raw::Target) -> Target {
    let kind = if t.kind.iter().any(|k| k == "bin") {
        TargetKind::Bin
    } else if t.kind.iter().any(|k| k == "lib") {
        TargetKind::Lib
    } else {
        TargetKind::Other
    };
    Target {
        name: t.name.clone(),
        kind,
        src_path: PathBuf::from(&t.src_path),
    }
}

fn dep_from_raw(d: &raw::Dependency) -> Dependency {
    let kind = match d.kind.as_deref() {
        Some("dev") => DepKind::Dev,
        Some("build") => DepKind::Build,
        _ => DepKind::Normal,
    };
    Dependency {
        name: d.name.clone(),
        rename: d.rename.clone(),
        req: d.req.clone(),
        features: d.features.clone(),
        kind,
    }
}

/// Direct member→member edges (dev-only edges dropped, since they don't affect
/// a built artifact).
fn member_edges(resolve: &raw::Resolve, members: &HashSet<&str>) -> HashMap<String, Vec<String>> {
    let mut edges = HashMap::new();
    for node in &resolve.nodes {
        if !members.contains(node.id.as_str()) {
            continue;
        }
        let deps = node
            .deps
            .iter()
            .filter(|d| members.contains(d.pkg.as_str()))
            .filter(|d| {
                d.dep_kinds.is_empty()
                    || d.dep_kinds.iter().any(|k| k.kind.as_deref() != Some("dev"))
            })
            .map(|d| d.pkg.clone())
            .collect();
        edges.insert(node.id.clone(), deps);
    }
    edges
}

// --- cargo metadata JSON (only the fields we use) -----------------------

mod raw {
    use serde::Deserialize;

    #[derive(Deserialize)]
    pub struct Metadata {
        pub packages: Vec<Package>,
        pub workspace_members: Vec<String>,
        pub workspace_root: String,
        pub target_directory: String,
        pub resolve: Option<Resolve>,
    }

    #[derive(Deserialize)]
    pub struct Package {
        pub id: String,
        pub name: String,
        #[serde(default)]
        pub description: Option<String>,
        #[serde(default)]
        pub edition: Option<String>,
        #[serde(default)]
        pub license: Option<String>,
        pub manifest_path: String,
        pub targets: Vec<Target>,
        #[serde(default)]
        pub dependencies: Vec<Dependency>,
    }

    #[derive(Deserialize)]
    pub struct Target {
        pub name: String,
        pub kind: Vec<String>,
        pub src_path: String,
    }

    #[derive(Deserialize)]
    pub struct Dependency {
        pub name: String,
        pub req: String,
        #[serde(default)]
        pub features: Vec<String>,
        #[serde(default)]
        pub rename: Option<String>,
        #[serde(default)]
        pub kind: Option<String>,
    }

    #[derive(Deserialize)]
    pub struct Resolve {
        pub nodes: Vec<Node>,
    }

    #[derive(Deserialize)]
    pub struct Node {
        pub id: String,
        #[serde(default)]
        pub deps: Vec<NodeDep>,
    }

    #[derive(Deserialize)]
    pub struct NodeDep {
        pub pkg: String,
        #[serde(default)]
        pub dep_kinds: Vec<DepKind>,
    }

    #[derive(Deserialize)]
    pub struct DepKind {
        #[serde(default)]
        pub kind: Option<String>,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = r#"{
      "packages": [
        {"id":"core 0.1.0 (path+file:///core)","name":"core","description":null,
         "manifest_path":"/ws/core/Cargo.toml","edition":"2021","license":"MIT",
         "targets":[{"name":"core","kind":["lib"],"src_path":"/ws/core/src/lib.rs"}],
         "dependencies":[]},
        {"id":"app 0.1.0 (path+file:///app)","name":"app","description":"the app",
         "manifest_path":"/ws/app/Cargo.toml","edition":"2021","license":null,
         "targets":[{"name":"app","kind":["bin"],"src_path":"/ws/app/src/main.rs"}],
         "dependencies":[
           {"name":"core","req":"*","features":[],"rename":null,"kind":null},
           {"name":"serde","req":"^1","features":["derive"],"rename":null,"kind":null},
           {"name":"tempfile","req":"^3","features":[],"rename":null,"kind":"dev"}
         ]}
      ],
      "workspace_members": ["core 0.1.0 (path+file:///core)","app 0.1.0 (path+file:///app)"],
      "workspace_root":"/ws",
      "target_directory":"/ws/target",
      "resolve": {"nodes":[
        {"id":"core 0.1.0 (path+file:///core)","deps":[]},
        {"id":"app 0.1.0 (path+file:///app)","deps":[
          {"pkg":"core 0.1.0 (path+file:///core)","dep_kinds":[{"kind":null}]}
        ]}
      ]}
    }"#;

    fn ws() -> Workspace {
        parse(FIXTURE.as_bytes()).expect("fixture parses")
    }

    #[test]
    fn discovers_members_and_roots() {
        let ws = ws();
        assert_eq!(ws.members.len(), 2);
        let app = ws.member("app").expect("app present");
        assert_eq!(app.crate_root().unwrap(), Path::new("/ws/app/src/main.rs"));
        assert_eq!(app.src_dir().unwrap(), Path::new("/ws/app/src"));
        assert_eq!(app.description.as_deref(), Some("the app"));
        assert_eq!(app.edition, "2021");
        let core = ws.member("core").expect("core present");
        assert_eq!(core.crate_root().unwrap(), Path::new("/ws/core/src/lib.rs"));
        assert_eq!(core.license.as_deref(), Some("MIT"));
    }

    #[test]
    fn dependency_kinds_and_idents() {
        let app = ws();
        let app = app.member("app").unwrap();
        let serde = app.dependencies.iter().find(|d| d.name == "serde").unwrap();
        assert_eq!(serde.features, vec!["derive"]);
        assert!(serde.is_normal());
        assert_eq!(serde.extern_ident(), "serde");
        let dev = app
            .dependencies
            .iter()
            .find(|d| d.name == "tempfile")
            .unwrap();
        assert_eq!(dev.kind, DepKind::Dev);
        assert!(!dev.is_normal());
    }

    #[test]
    fn linked_members_follows_the_graph() {
        let ws = ws();
        let app_id = "app 0.1.0 (path+file:///app)";
        let linked = ws.linked_members(app_id).expect("resolve graph present");
        assert!(linked.contains(app_id));
        assert!(linked.contains("core 0.1.0 (path+file:///core)"));
    }

    #[test]
    fn linked_members_is_none_without_resolve() {
        // Same fixture minus the resolve graph.
        let no_resolve = FIXTURE.replace(
            r#"      "resolve": {"nodes":[
        {"id":"core 0.1.0 (path+file:///core)","deps":[]},
        {"id":"app 0.1.0 (path+file:///app)","deps":[
          {"pkg":"core 0.1.0 (path+file:///core)","dep_kinds":[{"kind":null}]}
        ]}
      ]}"#,
            r#"      "resolve": null"#,
        );
        let ws = parse(no_resolve.as_bytes()).unwrap();
        assert!(ws.linked_members("app 0.1.0 (path+file:///app)").is_none());
    }
}
