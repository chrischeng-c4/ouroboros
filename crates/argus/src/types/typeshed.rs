//! Bundled typeshed stubs for standard library modules
//!
//! This module provides type information for common Python stdlib modules.

use super::imports::ModuleInfo;
use super::ty::Type;

/// Create os module stub
pub fn create_os_stub() -> ModuleInfo {
    let mut info = ModuleInfo::new("os");

    // Path operations
    info.exports.insert("getcwd".to_string(), Type::callable(vec![], Type::Str));
    info.exports.insert("chdir".to_string(), Type::callable(vec![Type::Str], Type::None));
    info.exports.insert("listdir".to_string(), Type::callable(vec![Type::Str], Type::list(Type::Str)));
    info.exports.insert("mkdir".to_string(), Type::callable(vec![Type::Str], Type::None));
    info.exports.insert("makedirs".to_string(), Type::callable(vec![Type::Str], Type::None));
    info.exports.insert("remove".to_string(), Type::callable(vec![Type::Str], Type::None));
    info.exports.insert("rmdir".to_string(), Type::callable(vec![Type::Str], Type::None));
    info.exports.insert("rename".to_string(), Type::callable(vec![Type::Str, Type::Str], Type::None));
    info.exports.insert("stat".to_string(), Type::callable(vec![Type::Str], Type::Any));
    info.exports.insert("walk".to_string(), Type::callable(vec![Type::Str], Type::Any));

    // Environment
    info.exports.insert("environ".to_string(), Type::dict(Type::Str, Type::Str));
    info.exports.insert("getenv".to_string(), Type::callable(vec![Type::Str], Type::optional(Type::Str)));
    info.exports.insert("putenv".to_string(), Type::callable(vec![Type::Str, Type::Str], Type::None));

    // Process
    info.exports.insert("getpid".to_string(), Type::callable(vec![], Type::Int));
    info.exports.insert("getppid".to_string(), Type::callable(vec![], Type::Int));
    info.exports.insert("system".to_string(), Type::callable(vec![Type::Str], Type::Int));
    info.exports.insert("popen".to_string(), Type::callable(vec![Type::Str], Type::Any));

    // Path separator
    info.exports.insert("sep".to_string(), Type::Str);
    info.exports.insert("linesep".to_string(), Type::Str);
    info.exports.insert("pathsep".to_string(), Type::Str);
    info.exports.insert("name".to_string(), Type::Str);

    info
}

/// Create os.path module stub
pub fn create_os_path_stub() -> ModuleInfo {
    let mut info = ModuleInfo::new("os.path");

    info.exports.insert("join".to_string(), Type::callable(vec![Type::Str, Type::Str], Type::Str));
    info.exports.insert("exists".to_string(), Type::callable(vec![Type::Str], Type::Bool));
    info.exports.insert("isfile".to_string(), Type::callable(vec![Type::Str], Type::Bool));
    info.exports.insert("isdir".to_string(), Type::callable(vec![Type::Str], Type::Bool));
    info.exports.insert("isabs".to_string(), Type::callable(vec![Type::Str], Type::Bool));
    info.exports.insert("islink".to_string(), Type::callable(vec![Type::Str], Type::Bool));
    info.exports.insert("basename".to_string(), Type::callable(vec![Type::Str], Type::Str));
    info.exports.insert("dirname".to_string(), Type::callable(vec![Type::Str], Type::Str));
    info.exports.insert("split".to_string(), Type::callable(vec![Type::Str], Type::Tuple(vec![Type::Str, Type::Str])));
    info.exports.insert("splitext".to_string(), Type::callable(vec![Type::Str], Type::Tuple(vec![Type::Str, Type::Str])));
    info.exports.insert("abspath".to_string(), Type::callable(vec![Type::Str], Type::Str));
    info.exports.insert("realpath".to_string(), Type::callable(vec![Type::Str], Type::Str));
    info.exports.insert("normpath".to_string(), Type::callable(vec![Type::Str], Type::Str));
    info.exports.insert("expanduser".to_string(), Type::callable(vec![Type::Str], Type::Str));
    info.exports.insert("expandvars".to_string(), Type::callable(vec![Type::Str], Type::Str));
    info.exports.insert("getsize".to_string(), Type::callable(vec![Type::Str], Type::Int));
    info.exports.insert("getmtime".to_string(), Type::callable(vec![Type::Str], Type::Float));
    info.exports.insert("getctime".to_string(), Type::callable(vec![Type::Str], Type::Float));
    info.exports.insert("getatime".to_string(), Type::callable(vec![Type::Str], Type::Float));

    info
}

/// Create sys module stub
pub fn create_sys_stub() -> ModuleInfo {
    let mut info = ModuleInfo::new("sys");

    // Streams
    info.exports.insert("stdin".to_string(), Type::Any);
    info.exports.insert("stdout".to_string(), Type::Any);
    info.exports.insert("stderr".to_string(), Type::Any);

    // Arguments
    info.exports.insert("argv".to_string(), Type::list(Type::Str));

    // Paths
    info.exports.insert("path".to_string(), Type::list(Type::Str));
    info.exports.insert("modules".to_string(), Type::dict(Type::Str, Type::Any));

    // Version info
    info.exports.insert("version".to_string(), Type::Str);
    info.exports.insert("version_info".to_string(), Type::Tuple(vec![
        Type::Int, Type::Int, Type::Int, Type::Str, Type::Int,
    ]));
    info.exports.insert("platform".to_string(), Type::Str);
    info.exports.insert("executable".to_string(), Type::Str);
    info.exports.insert("prefix".to_string(), Type::Str);

    // Functions
    info.exports.insert("exit".to_string(), Type::callable(vec![Type::Int], Type::Never));
    info.exports.insert("getrecursionlimit".to_string(), Type::callable(vec![], Type::Int));
    info.exports.insert("setrecursionlimit".to_string(), Type::callable(vec![Type::Int], Type::None));
    info.exports.insert("getsizeof".to_string(), Type::callable(vec![Type::Any], Type::Int));

    // Numeric limits
    info.exports.insert("maxsize".to_string(), Type::Int);
    info.exports.insert("float_info".to_string(), Type::Any);
    info.exports.insert("int_info".to_string(), Type::Any);

    info
}

/// Create io module stub
pub fn create_io_stub() -> ModuleInfo {
    let mut info = ModuleInfo::new("io");

    // Base classes
    info.exports.insert("IOBase".to_string(), Type::ClassType {
        name: "IOBase".to_string(),
        module: Some("io".to_string()),
    });
    info.exports.insert("RawIOBase".to_string(), Type::ClassType {
        name: "RawIOBase".to_string(),
        module: Some("io".to_string()),
    });
    info.exports.insert("BufferedIOBase".to_string(), Type::ClassType {
        name: "BufferedIOBase".to_string(),
        module: Some("io".to_string()),
    });
    info.exports.insert("TextIOBase".to_string(), Type::ClassType {
        name: "TextIOBase".to_string(),
        module: Some("io".to_string()),
    });

    // Concrete classes
    info.exports.insert("FileIO".to_string(), Type::ClassType {
        name: "FileIO".to_string(),
        module: Some("io".to_string()),
    });
    info.exports.insert("BytesIO".to_string(), Type::ClassType {
        name: "BytesIO".to_string(),
        module: Some("io".to_string()),
    });
    info.exports.insert("StringIO".to_string(), Type::ClassType {
        name: "StringIO".to_string(),
        module: Some("io".to_string()),
    });
    info.exports.insert("BufferedReader".to_string(), Type::ClassType {
        name: "BufferedReader".to_string(),
        module: Some("io".to_string()),
    });
    info.exports.insert("BufferedWriter".to_string(), Type::ClassType {
        name: "BufferedWriter".to_string(),
        module: Some("io".to_string()),
    });
    info.exports.insert("TextIOWrapper".to_string(), Type::ClassType {
        name: "TextIOWrapper".to_string(),
        module: Some("io".to_string()),
    });

    // Functions
    info.exports.insert("open".to_string(), Type::callable(
        vec![Type::Str, Type::Str],
        Type::Any,
    ));

    // Constants
    info.exports.insert("DEFAULT_BUFFER_SIZE".to_string(), Type::Int);
    info.exports.insert("SEEK_SET".to_string(), Type::Int);
    info.exports.insert("SEEK_CUR".to_string(), Type::Int);
    info.exports.insert("SEEK_END".to_string(), Type::Int);

    info
}

/// Create re module stub
pub fn create_re_stub() -> ModuleInfo {
    let mut info = ModuleInfo::new("re");

    // Pattern and Match types
    info.exports.insert("Pattern".to_string(), Type::ClassType {
        name: "Pattern".to_string(),
        module: Some("re".to_string()),
    });
    info.exports.insert("Match".to_string(), Type::ClassType {
        name: "Match".to_string(),
        module: Some("re".to_string()),
    });

    // Functions
    info.exports.insert("compile".to_string(), Type::callable(
        vec![Type::Str],
        Type::Instance {
            name: "Pattern".to_string(),
            module: Some("re".to_string()),
            type_args: vec![Type::Str],
        },
    ));
    info.exports.insert("match".to_string(), Type::callable(
        vec![Type::Str, Type::Str],
        Type::optional(Type::Instance {
            name: "Match".to_string(),
            module: Some("re".to_string()),
            type_args: vec![Type::Str],
        }),
    ));
    info.exports.insert("search".to_string(), Type::callable(
        vec![Type::Str, Type::Str],
        Type::optional(Type::Instance {
            name: "Match".to_string(),
            module: Some("re".to_string()),
            type_args: vec![Type::Str],
        }),
    ));
    info.exports.insert("findall".to_string(), Type::callable(
        vec![Type::Str, Type::Str],
        Type::list(Type::Str),
    ));
    info.exports.insert("finditer".to_string(), Type::callable(
        vec![Type::Str, Type::Str],
        Type::Any, // Iterator[Match]
    ));
    info.exports.insert("sub".to_string(), Type::callable(
        vec![Type::Str, Type::Str, Type::Str],
        Type::Str,
    ));
    info.exports.insert("subn".to_string(), Type::callable(
        vec![Type::Str, Type::Str, Type::Str],
        Type::Tuple(vec![Type::Str, Type::Int]),
    ));
    info.exports.insert("split".to_string(), Type::callable(
        vec![Type::Str, Type::Str],
        Type::list(Type::Str),
    ));
    info.exports.insert("escape".to_string(), Type::callable(
        vec![Type::Str],
        Type::Str,
    ));

    // Flags
    info.exports.insert("IGNORECASE".to_string(), Type::Int);
    info.exports.insert("I".to_string(), Type::Int);
    info.exports.insert("MULTILINE".to_string(), Type::Int);
    info.exports.insert("M".to_string(), Type::Int);
    info.exports.insert("DOTALL".to_string(), Type::Int);
    info.exports.insert("S".to_string(), Type::Int);
    info.exports.insert("VERBOSE".to_string(), Type::Int);
    info.exports.insert("X".to_string(), Type::Int);
    info.exports.insert("ASCII".to_string(), Type::Int);
    info.exports.insert("A".to_string(), Type::Int);

    info
}

/// Create json module stub
pub fn create_json_stub() -> ModuleInfo {
    let mut info = ModuleInfo::new("json");

    info.exports.insert("dumps".to_string(), Type::callable(
        vec![Type::Any],
        Type::Str,
    ));
    info.exports.insert("loads".to_string(), Type::callable(
        vec![Type::Str],
        Type::Any,
    ));
    info.exports.insert("dump".to_string(), Type::callable(
        vec![Type::Any, Type::Any],
        Type::None,
    ));
    info.exports.insert("load".to_string(), Type::callable(
        vec![Type::Any],
        Type::Any,
    ));

    info.exports.insert("JSONEncoder".to_string(), Type::ClassType {
        name: "JSONEncoder".to_string(),
        module: Some("json".to_string()),
    });
    info.exports.insert("JSONDecoder".to_string(), Type::ClassType {
        name: "JSONDecoder".to_string(),
        module: Some("json".to_string()),
    });
    info.exports.insert("JSONDecodeError".to_string(), Type::ClassType {
        name: "JSONDecodeError".to_string(),
        module: Some("json".to_string()),
    });

    info
}

/// Create pathlib module stub
pub fn create_pathlib_stub() -> ModuleInfo {
    let mut info = ModuleInfo::new("pathlib");

    let path_type = Type::ClassType {
        name: "Path".to_string(),
        module: Some("pathlib".to_string()),
    };

    info.exports.insert("Path".to_string(), path_type.clone());
    info.exports.insert("PurePath".to_string(), Type::ClassType {
        name: "PurePath".to_string(),
        module: Some("pathlib".to_string()),
    });
    info.exports.insert("PurePosixPath".to_string(), Type::ClassType {
        name: "PurePosixPath".to_string(),
        module: Some("pathlib".to_string()),
    });
    info.exports.insert("PureWindowsPath".to_string(), Type::ClassType {
        name: "PureWindowsPath".to_string(),
        module: Some("pathlib".to_string()),
    });
    info.exports.insert("PosixPath".to_string(), Type::ClassType {
        name: "PosixPath".to_string(),
        module: Some("pathlib".to_string()),
    });
    info.exports.insert("WindowsPath".to_string(), Type::ClassType {
        name: "WindowsPath".to_string(),
        module: Some("pathlib".to_string()),
    });

    info
}

/// Create functools module stub
pub fn create_functools_stub() -> ModuleInfo {
    let mut info = ModuleInfo::new("functools");

    info.exports.insert("reduce".to_string(), Type::callable(
        vec![Type::Any, Type::Any],
        Type::Any,
    ));
    info.exports.insert("partial".to_string(), Type::callable(
        vec![Type::Any],
        Type::Any,
    ));
    info.exports.insert("wraps".to_string(), Type::callable(
        vec![Type::Any],
        Type::Any,
    ));
    info.exports.insert("lru_cache".to_string(), Type::callable(
        vec![],
        Type::Any,
    ));
    info.exports.insert("cache".to_string(), Type::callable(
        vec![Type::Any],
        Type::Any,
    ));
    info.exports.insert("cached_property".to_string(), Type::callable(
        vec![Type::Any],
        Type::Any,
    ));
    info.exports.insert("total_ordering".to_string(), Type::callable(
        vec![Type::Any],
        Type::Any,
    ));
    info.exports.insert("cmp_to_key".to_string(), Type::callable(
        vec![Type::Any],
        Type::Any,
    ));

    info
}

/// Create itertools module stub
pub fn create_itertools_stub() -> ModuleInfo {
    let mut info = ModuleInfo::new("itertools");

    // Infinite iterators
    info.exports.insert("count".to_string(), Type::callable(vec![Type::Int], Type::Any));
    info.exports.insert("cycle".to_string(), Type::callable(vec![Type::Any], Type::Any));
    info.exports.insert("repeat".to_string(), Type::callable(vec![Type::Any], Type::Any));

    // Combinatoric iterators
    info.exports.insert("product".to_string(), Type::callable(vec![Type::Any], Type::Any));
    info.exports.insert("permutations".to_string(), Type::callable(vec![Type::Any], Type::Any));
    info.exports.insert("combinations".to_string(), Type::callable(vec![Type::Any, Type::Int], Type::Any));
    info.exports.insert("combinations_with_replacement".to_string(), Type::callable(vec![Type::Any, Type::Int], Type::Any));

    // Terminating iterators
    info.exports.insert("chain".to_string(), Type::callable(vec![Type::Any], Type::Any));
    info.exports.insert("compress".to_string(), Type::callable(vec![Type::Any, Type::Any], Type::Any));
    info.exports.insert("dropwhile".to_string(), Type::callable(vec![Type::Any, Type::Any], Type::Any));
    info.exports.insert("takewhile".to_string(), Type::callable(vec![Type::Any, Type::Any], Type::Any));
    info.exports.insert("groupby".to_string(), Type::callable(vec![Type::Any], Type::Any));
    info.exports.insert("islice".to_string(), Type::callable(vec![Type::Any, Type::Int], Type::Any));
    info.exports.insert("starmap".to_string(), Type::callable(vec![Type::Any, Type::Any], Type::Any));
    info.exports.insert("tee".to_string(), Type::callable(vec![Type::Any], Type::Any));
    info.exports.insert("zip_longest".to_string(), Type::callable(vec![Type::Any], Type::Any));
    info.exports.insert("filterfalse".to_string(), Type::callable(vec![Type::Any, Type::Any], Type::Any));
    info.exports.insert("accumulate".to_string(), Type::callable(vec![Type::Any], Type::Any));

    info
}

/// Create datetime module stub
pub fn create_datetime_stub() -> ModuleInfo {
    let mut info = ModuleInfo::new("datetime");

    info.exports.insert("date".to_string(), Type::ClassType {
        name: "date".to_string(),
        module: Some("datetime".to_string()),
    });
    info.exports.insert("time".to_string(), Type::ClassType {
        name: "time".to_string(),
        module: Some("datetime".to_string()),
    });
    info.exports.insert("datetime".to_string(), Type::ClassType {
        name: "datetime".to_string(),
        module: Some("datetime".to_string()),
    });
    info.exports.insert("timedelta".to_string(), Type::ClassType {
        name: "timedelta".to_string(),
        module: Some("datetime".to_string()),
    });
    info.exports.insert("timezone".to_string(), Type::ClassType {
        name: "timezone".to_string(),
        module: Some("datetime".to_string()),
    });
    info.exports.insert("tzinfo".to_string(), Type::ClassType {
        name: "tzinfo".to_string(),
        module: Some("datetime".to_string()),
    });

    // Constants
    info.exports.insert("MINYEAR".to_string(), Type::Int);
    info.exports.insert("MAXYEAR".to_string(), Type::Int);

    info
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_os_stub() {
        let os = create_os_stub();
        assert!(os.exports.contains_key("getcwd"));
        assert!(os.exports.contains_key("environ"));
        assert!(os.exports.contains_key("sep"));
    }

    #[test]
    fn test_sys_stub() {
        let sys = create_sys_stub();
        assert!(sys.exports.contains_key("argv"));
        assert!(sys.exports.contains_key("path"));
        assert!(sys.exports.contains_key("exit"));
    }

    #[test]
    fn test_re_stub() {
        let re = create_re_stub();
        assert!(re.exports.contains_key("compile"));
        assert!(re.exports.contains_key("match"));
        assert!(re.exports.contains_key("IGNORECASE"));
    }

    #[test]
    fn test_json_stub() {
        let json = create_json_stub();
        assert!(json.exports.contains_key("dumps"));
        assert!(json.exports.contains_key("loads"));
    }

    #[test]
    fn test_datetime_stub() {
        let dt = create_datetime_stub();
        assert!(dt.exports.contains_key("datetime"));
        assert!(dt.exports.contains_key("date"));
        assert!(dt.exports.contains_key("timedelta"));
    }
}
