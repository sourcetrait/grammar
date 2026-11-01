use crate::*;

#[derive(Debug, Clone, ser::Serialize, ser::Deserialize)]
pub struct RigIndex {
    pub source_path: PathBuf,
    #[serde(default)]
    pub functions: Vec<IndexFunction>,
    #[serde(default)]
    pub modules: Vec<IndexModule>,
}

/// A call's index entry: its name and its contract's types.
#[derive(Debug, Clone, ser::Serialize, ser::Deserialize)]
pub struct IndexFunction {
    pub name: String,
    #[serde(deserialize_with = "stored_type")]
    pub data_type: NuType,
    #[serde(deserialize_with = "stored_type")]
    pub result_type: NuType,
    pub takes_timeout: bool,
}

/// A stored type: its text, or the structure an earlier build wrote.
#[derive(ser::Deserialize)]
#[serde(untagged)]
enum StoredType {
    Text(NuType),
    Earlier(EarlierType),
}

/// An index type as an earlier build wrote it, read so its index loads
/// until the rig is committed again.
#[derive(ser::Deserialize)]
enum EarlierType {
    Scalar(EarlierScalar),
    Nothing,
    Record { fields: Vec<EarlierField> },
    Oneof { members: Vec<EarlierType> },
    Table { columns: Vec<EarlierField> },
    List { element: Box<EarlierType> },
}

/// An earlier build's scalar.
#[derive(ser::Deserialize)]
enum EarlierScalar {
    Int,
    Float,
    String,
    Bool,
    Datetime,
    Duration,
    Filesize,
    Binary,
    Range,
    Number,
    Glob,
    CellPath,
    Path,
    Directory,
}

/// An earlier build's field or column.
#[derive(ser::Deserialize)]
struct EarlierField {
    name: String,
    typedef: EarlierType,
}

impl From<EarlierType> for NuType {
    fn from(earlier: EarlierType) -> Self {
        let fields = |fields: Vec<EarlierField>| -> Vec<NuField> {
            fields.into_iter().map(|f| NuField::new(f.name, f.typedef.into())).collect()
        };
        match earlier {
            EarlierType::Nothing => NuType::Nothing,
            EarlierType::Record { fields: f } => NuType::Record(fields(f)),
            EarlierType::Table { columns } => NuType::Table(fields(columns)),
            EarlierType::Oneof { members } => NuType::OneOf(members.into_iter().map(Into::into).collect()),
            EarlierType::List { element } => NuType::List(Box::new((*element).into())),
            EarlierType::Scalar(scalar) => match scalar {
                EarlierScalar::Int => NuType::Int,
                EarlierScalar::Float => NuType::Float,
                EarlierScalar::String => NuType::String,
                EarlierScalar::Bool => NuType::Bool,
                EarlierScalar::Datetime => NuType::Date,
                EarlierScalar::Duration => NuType::Duration,
                EarlierScalar::Filesize => NuType::Filesize,
                EarlierScalar::Binary => NuType::Binary,
                EarlierScalar::Range => NuType::Range,
                EarlierScalar::Number => NuType::Number,
                EarlierScalar::Glob => NuType::Glob,
                EarlierScalar::CellPath => NuType::CellPath,
                EarlierScalar::Path => NuType::Path,
                EarlierScalar::Directory => NuType::Directory,
            },
        }
    }
}

/// An index entry's type, its text or an earlier build's structure.
fn stored_type<'de, D: ser::Deserializer<'de>>(deserializer: D) -> Result<NuType, D::Error> {
    Ok(match <StoredType as ser::Deserialize>::deserialize(deserializer)? {
        StoredType::Text(ty) => ty,
        StoredType::Earlier(earlier) => earlier.into(),
    })
}

#[derive(Debug, Clone, ser::Serialize, ser::Deserialize)]
pub struct IndexModule {
    pub name: String,
    #[serde(default)]
    pub functions: Vec<IndexFunction>,
    #[serde(default)]
    pub modules: Vec<IndexModule>,
}

/// A lib's index: the source it was committed from.
#[derive(Debug, Clone, ser::Serialize, ser::Deserialize)]
pub struct LibIndex {
    pub source_path: PathBuf,
}

/// A rig's index read for its source alone, whichever build wrote its entries.
#[derive(ser::Deserialize)]
struct IndexSource {
    source_path: PathBuf,
}

/// The rig's index, under its kind directory.
pub(crate) const META_FILE: &str = ".meta/rig.nuon";

/// The lib's index, under its kind directory.
pub(crate) const LIB_META_FILE: &str = ".meta/lib.nuon";

/// The kind holding an equip's calls.
pub(crate) const KIND_RIG: &str = "rig";

/// The kind holding an equip's plain modules.
pub(crate) const KIND_LIB: &str = "lib";

/// The kinds the MCP installs: a lib first, since its rig may import it.
pub(crate) const EQUIP_KINDS: [&str; 2] = [KIND_LIB, KIND_RIG];

/// The directory that names an equip, in its source path and the namespace.
pub(crate) const EQUIP_DIR: &str = "equip";

/// The environment variable that overrides the deploy's equip directory.
pub(crate) const EQUIP_DIR_ENV: &str = "GRAMMAR_MCP_EQUIP_DIR";


pub(crate) fn keypair_dir() -> PathBuf {
    data_home().join("keypair")
}

pub(crate) fn private_key_path() -> PathBuf {
    keypair_dir().join("id_grammar")
}

pub(crate) fn public_key_path() -> PathBuf {
    keypair_dir().join("id_grammar.pub")
}

pub(crate) fn allowed_signers_path() -> PathBuf {
    keypair_dir().join("allowed_signers")
}

/// The namespace's signed repository of installed equips.
pub fn equip_root() -> PathBuf {
    data_home().join(EQUIP_DIR)
}

/// An installed equip's directory, its kinds beneath it.
pub(crate) fn equip_dir(equip: &str) -> PathBuf {
    equip_root().join(equip)
}

/// An installed equip's rig.
pub fn rig_dir(equip: &str) -> PathBuf {
    equip_dir(equip).join(KIND_RIG)
}

/// An installed equip's lib.
pub fn lib_dir(equip: &str) -> PathBuf {
    equip_dir(equip).join(KIND_LIB)
}

pub(crate) fn is_valid_equip(equip: &str) -> bool {
    match equip.split_once('/') {
        Some((author, name)) => {
            !name.contains('/')
                && is_valid_ident(author)
                && !is_reserved_term(author)
                && is_valid_ident(name)
                && !is_reserved_term(name)
        }
        None => false,
    }
}

pub fn rig_meta_path(equip: &str) -> PathBuf {
    rig_dir(equip).join(META_FILE)
}

pub(crate) fn lib_meta_path(equip: &str) -> PathBuf {
    lib_dir(equip).join(LIB_META_FILE)
}

pub(crate) fn rig_meta_dir(equip: &str) -> PathBuf {
    rig_dir(equip).join(".meta")
}

pub(crate) fn rig_docs_dir(equip: &str) -> PathBuf {
    rig_meta_dir(equip).join("docs")
}

/// The equip a source path names: its last two components under `equip/`.
pub fn equip_namepath_of(source_dir: &Path) -> Result<String, EngineError> {
    let invalid = |reason: &str| EngineError::EquipInvalidSource {
        source_dir: source_dir.display().to_string(),
        reason: reason.to_string(),
    };
    let mut parts: Vec<&str> = Vec::new();
    for component in source_dir.components() {
        match component {
            std::path::Component::Normal(part) => match part.to_str() {
                Some(s) => parts.push(s),
                None => return Err(invalid("the path is not valid UTF-8")),
            },
            std::path::Component::RootDir => {}
            _ => return Err(invalid("the path must not hold `.` or `..`")),
        }
    }
    let [.., marker, author, name] = parts.as_slice() else {
        return Err(invalid("source_dir must be an `equip/<author>/<name>` directory"));
    };
    if *marker != EQUIP_DIR {
        return Err(invalid("source_dir must be an `equip/<author>/<name>` directory"));
    }
    let equip = format!("{author}/{name}");
    check_equip_name(&equip)?;
    Ok(equip)
}

/// Refuse an equip name that is malformed, reserved, or denied.
pub(crate) fn check_equip_name(equip: &str) -> Result<(), EngineError> {
    let Some((author, name)) = equip.split_once('/').filter(|(_, n)| !n.contains('/')) else {
        return Err(EngineError::EquipInvalidName {
            equip: equip.to_string(),
            reason: "an equip is the compound `<author>/<name>`".to_string(),
        });
    };
    for seg in [author, name] {
        if !is_valid_ident(seg) || is_reserved_term(seg) {
            return Err(EngineError::EquipInvalidName {
                equip: equip.to_string(),
                reason: "author and name must match [a-zA-Z_][a-zA-Z0-9_-]* and not be the reserved `main`"
                    .to_string(),
            });
        }
    }
    if NAME_DENYLIST.contains(&name) || HELP_WORDS.contains(&author) || HELP_WORDS.contains(&name)
    {
        return Err(EngineError::EquipNameDenied {
            equip: equip.to_string(),
        });
    }
    Ok(())
}

/// Refuse an equip that is not installed, or one with no rig.
pub fn require_rig(equip: &str) -> Result<(), EngineError> {
    if !equip_dir(equip).exists() {
        return Err(EngineError::EquipNotRegistered {
            equip: equip.to_string(),
        });
    }
    if !rig_meta_path(equip).exists() {
        return Err(EngineError::EquipNoRig {
            equip: equip.to_string(),
        });
    }
    Ok(())
}


pub struct RigLocks {
    map: tk::AsyncMutex<HashMap<String, Arc<tk::AsyncRwLock<()>>>>,
}

impl Default for RigLocks {
    fn default() -> Self {
        Self::new()
    }
}

impl RigLocks {
    pub fn new() -> Self {
        Self {
            map: tk::AsyncMutex::new(HashMap::new()),
        }
    }

    pub async fn hydrate_from_disk(&self) -> io::Result<()> {
        let mut map = self.map.lock().await;
        for equip in registered_equip_names() {
            map.entry(equip)
                .or_insert_with(|| Arc::new(tk::AsyncRwLock::new(())));
        }
        Ok(())
    }

    pub async fn register(
        &self,
        name: &str,
    ) -> Result<Arc<tk::AsyncRwLock<()>>, AlreadyRegistered> {
        let mut map = self.map.lock().await;
        use std::collections::hash_map::Entry;
        match map.entry(name.to_string()) {
            Entry::Occupied(_) => Err(AlreadyRegistered),
            Entry::Vacant(v) => {
                let lock = Arc::new(tk::AsyncRwLock::new(()));
                v.insert(lock.clone());
                Ok(lock)
            }
        }
    }

    pub async fn lookup(&self, name: &str) -> Option<Arc<tk::AsyncRwLock<()>>> {
        let map = self.map.lock().await;
        map.get(name).cloned()
    }

    pub async fn unregister(&self, name: &str) {
        let mut map = self.map.lock().await;
        map.remove(name);
    }
}

pub struct AlreadyRegistered;


/// Serializes mutations of the ONE equip repo and its namespace meta.
static EQUIP_REPO_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub fn equip_repo_lock() -> std::sync::MutexGuard<'static, ()> {
    EQUIP_REPO_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}


const KEY_COMMENT: &str = "grammar@localhost";

pub(crate) fn ensure_keypair() -> io::Result<()> {
    let dir = keypair_dir();
    fs::create_dir_all(&dir)?;
    let priv_path = private_key_path();
    if !priv_path.exists() {
        let status = process::Command::new("ssh-keygen")
            .arg("-q")
            .arg("-t")
            .arg("ed25519")
            .arg("-f")
            .arg(&priv_path)
            .arg("-N")
            .arg("")
            .arg("-C")
            .arg(KEY_COMMENT)
            .status()
            .map_err(|e| io::Error::other(format!("ssh-keygen: {e}")))?;
        if !status.success() {
            return Err(io::Error::other(format!(
                "ssh-keygen failed with status {status}",
            )));
        }
    }
    let pub_key = fs::read_to_string(public_key_path())?;
    let allowed = format!("{} {}", KEY_COMMENT, pub_key.trim());
    fs::write(allowed_signers_path(), allowed.as_bytes())?;
    Ok(())
}

/// Make the namespace's equip repository when absent; true when this call made it.
pub(crate) fn ensure_equip_repo() -> io::Result<bool> {
    let dir = equip_root();
    fs::create_dir_all(&dir)?;
    let git_dir = dir.join(".git");
    if git_dir.exists() {
        configure_repo(&dir)?;
        return Ok(false);
    }
    run_git(&dir, &["init", "-b", "main"])?;
    configure_repo(&dir)?;
    run_git(
        &dir,
        &["commit", "--allow-empty", "-m", "init equip repo"],
    )?;
    Ok(true)
}

fn configure_repo(dir: &std::path::Path) -> io::Result<()> {
    let priv_path = private_key_path();
    let priv_path_str = priv_path
        .to_str()
        .ok_or_else(|| io::Error::other("private key path is not valid UTF-8"))?;
    let allowed_path = allowed_signers_path();
    let allowed_path_str = allowed_path
        .to_str()
        .ok_or_else(|| io::Error::other("allowed_signers path is not valid UTF-8"))?;
    let pairs: [(&str, &str); 6] = [
        ("user.name", "grammar"),
        ("user.email", KEY_COMMENT),
        ("gpg.format", "ssh"),
        ("user.signingKey", priv_path_str),
        ("gpg.ssh.allowedSignersFile", allowed_path_str),
        ("commit.gpgSign", "true"),
    ];
    for (k, v) in pairs {
        run_git(dir, &["config", k, v])?;
    }
    Ok(())
}

pub(crate) fn run_git(dir: &std::path::Path, args: &[&str]) -> io::Result<()> {
    let out = process::Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .map_err(|e| io::Error::other(format!("git: {e}")))?;
    if !out.status.success() {
        return Err(io::Error::other(format!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr),
        )));
    }
    Ok(())
}


pub(crate) fn is_valid_ident(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    if s == "mod" {
        return false;
    }
    let mut chars = s.chars();
    let first = chars.next().expect("non-empty");
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

pub(crate) fn is_valid_module_path(s: &str) -> bool {
    if s.is_empty() {
        return true;
    }
    if s.starts_with('/') || s.ends_with('/') || s.contains("//") {
        return false;
    }
    s.split('/').all(is_valid_ident)
}

/// Render a kind's index as NUON - the house format for anything we persist.
pub(crate) fn meta_to_nuon<T: ser::Serialize>(meta: &T) -> Result<String, String> {
    nuval::to_nuon(meta, NuonStyle::Default).map_err(|e| e.to_string())
}

/// Parse a kind's index back out of NUON, the mirror of `meta_to_nuon`.
fn meta_from_nuon<T: ser::DeserializeOwned>(text: &str) -> Result<T, String> {
    nuval::from_nuon(text).map_err(|e| e.to_string())
}

pub(crate) fn index_to_nuon(index: &RigIndex) -> Result<String, String> {
    meta_to_nuon(index)
}

pub fn index_from_nuon(text: &str) -> Result<RigIndex, String> {
    meta_from_nuon(text)
}

pub fn load_index(rig: &str) -> io::Result<RigIndex> {
    let text = fs::read_to_string(rig_meta_path(rig))?;
    index_from_nuon(&text)
        .map_err(|e| io::Error::other(format!("decode index for {rig}: {e}")))
}

/// A rig index's source path, its entries left unread.
pub(crate) fn index_source_from_nuon(text: &str) -> Result<PathBuf, String> {
    meta_from_nuon::<IndexSource>(text).map(|index| index.source_path)
}

/// The source a rig was committed from, whichever build wrote its index.
pub(crate) fn load_rig_source(rig: &str) -> io::Result<PathBuf> {
    let text = fs::read_to_string(rig_meta_path(rig))?;
    index_source_from_nuon(&text)
        .map_err(|e| io::Error::other(format!("decode index for {rig}: {e}")))
}

pub(crate) fn load_lib_index(equip: &str) -> io::Result<LibIndex> {
    let text = fs::read_to_string(lib_meta_path(equip))?;
    meta_from_nuon(&text)
        .map_err(|e| io::Error::other(format!("decode lib index for {equip}: {e}")))
}

/// Every installed equip's compound `<author>/<name>`, sorted.
pub(crate) fn registered_equip_names() -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    let Ok(authors) = fs::read_dir(equip_root()) else {
        return names;
    };
    for author_entry in authors.flatten() {
        if !author_entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let Ok(author) = author_entry.file_name().into_string() else {
            continue;
        };
        if author.starts_with('.') {
            continue;
        }
        let Ok(equips) = fs::read_dir(author_entry.path()) else {
            continue;
        };
        for equip_entry in equips.flatten() {
            if !equip_entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                continue;
            }
            let Ok(name) = equip_entry.file_name().into_string() else {
                continue;
            };
            let equip = format!("{author}/{name}");
            if rig_meta_path(&equip).exists() || lib_meta_path(&equip).exists() {
                names.push(equip);
            }
        }
    }
    names.sort();
    names
}

/// Every installed equip with a rig, sorted.
pub fn registered_rig_names() -> Vec<String> {
    registered_equip_names()
        .into_iter()
        .filter(|equip| rig_meta_path(equip).exists())
        .collect()
}

/// The equip store, its keypair and repository; true when this call made it.
pub fn ensure_equip_store() -> io::Result<bool> {
    let _repo = equip_repo_lock();
    ensure_keypair()?;
    ensure_equip_repo()
}

/// The deploy's equip directory: the override, else `<exe dir>/../equip`.
pub(crate) fn bundled_equip_dir() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os(EQUIP_DIR_ENV) {
        return Some(PathBuf::from(dir));
    }
    let exe = std::env::current_exe().ok()?;
    Some(exe.parent()?.parent()?.join(EQUIP_DIR))
}

/// Install each builtin the deploy holds; the names of those that installed.
pub fn install_builtins() -> Vec<&'static str> {
    let Some(dir) = bundled_equip_dir() else {
        return Vec::new();
    };
    let engine = LintEngine::new().current();
    let mut installed: Vec<&'static str> = Vec::new();
    for (name, author) in BUILTINS {
        let source = dir.join(author).join(name);
        if !source.is_dir() {
            continue;
        }
        let equip = format!("{author}/{name}");
        match install_equip(&equip, &source, &engine) {
            Ok(_) => installed.push(name),
            Err(e) => eprintln!(
                "grammar: builtin {equip} not installed from {}: {e:?}",
                source.display(),
            ),
        }
    }
    installed
}
