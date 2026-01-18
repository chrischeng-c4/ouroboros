//! Core type definitions for the Argus type system

use std::collections::HashMap;
use std::fmt;

/// Unique identifier for type variables
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TypeVarId(pub usize);

/// Parameter kind in function signatures
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamKind {
    /// Regular positional parameter
    Positional,
    /// Positional-only parameter (before /)
    PositionalOnly,
    /// Keyword-only parameter (after *)
    KeywordOnly,
    /// *args parameter
    VarPositional,
    /// **kwargs parameter
    VarKeyword,
}

/// Function parameter
#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: String,
    pub ty: Type,
    pub has_default: bool,
    pub kind: ParamKind,
}

/// Literal value for Literal types
#[derive(Debug, Clone, PartialEq)]
pub enum LiteralValue {
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    None,
}

/// The core Type enum representing all possible types
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    // === Primitive types ===
    /// The bottom type (NoReturn in Python, never in TypeScript)
    Never,
    /// None / null / unit type
    None,
    /// Boolean
    Bool,
    /// Integer (Python int, TypeScript number)
    Int,
    /// Floating point (Python float, TypeScript number)
    Float,
    /// String
    Str,
    /// Bytes (Python only)
    Bytes,

    // === Container types ===
    /// List / Array
    List(Box<Type>),
    /// Dictionary / Map / Object
    Dict(Box<Type>, Box<Type>),
    /// Set
    Set(Box<Type>),
    /// Tuple with fixed element types
    Tuple(Vec<Type>),

    // === Composite types ===
    /// Optional type (T | None)
    Optional(Box<Type>),
    /// Union type (T | U | V)
    Union(Vec<Type>),
    /// Intersection type (T & U) - TypeScript only
    Intersection(Vec<Type>),

    // === Callable types ===
    /// Function / Callable type
    Callable {
        params: Vec<Param>,
        ret: Box<Type>,
    },

    // === Class types ===
    /// Class / Interface / Struct instance
    Instance {
        name: String,
        module: Option<String>,
        type_args: Vec<Type>,
    },
    /// Class type itself (for type[T])
    ClassType {
        name: String,
        module: Option<String>,
    },

    // === Generic types ===
    /// Type variable (T, K, V, etc.)
    TypeVar {
        id: TypeVarId,
        name: String,
        bound: Option<Box<Type>>,
        constraints: Vec<Type>,
    },

    // === Protocol types ===
    /// Protocol type (structural subtyping)
    /// A type conforms to a Protocol if it has all required members
    Protocol {
        name: String,
        module: Option<String>,
        /// Required members (method/attribute name -> type)
        members: Vec<(String, Type)>,
    },

    // === TypedDict ===
    /// TypedDict type - dictionary with specific keys and types
    TypedDict {
        name: String,
        /// Fields with their types and whether they are required
        fields: Vec<(String, Type, bool)>, // (name, type, required)
        /// Whether all fields are required by default
        total: bool,
    },

    // === Special types ===
    /// Any type - disables type checking
    Any,
    /// Unknown type - not yet inferred
    Unknown,
    /// Literal type (Literal["foo"], Literal[42])
    Literal(LiteralValue),
    /// Self type (for method return types)
    SelfType,

    // === Error type ===
    /// Type error placeholder (allows continued analysis)
    Error,
}

impl Type {
    /// Create an Optional type
    pub fn optional(inner: Type) -> Self {
        Type::Optional(Box::new(inner))
    }

    /// Create a List type
    pub fn list(element: Type) -> Self {
        Type::List(Box::new(element))
    }

    /// Create a Dict type
    pub fn dict(key: Type, value: Type) -> Self {
        Type::Dict(Box::new(key), Box::new(value))
    }

    /// Create a Union type, flattening nested unions
    pub fn union(types: Vec<Type>) -> Self {
        let mut flattened = Vec::new();
        for ty in types {
            match ty {
                Type::Union(inner) => flattened.extend(inner),
                Type::Never => {} // Never is identity for union
                other => {
                    if !flattened.contains(&other) {
                        flattened.push(other);
                    }
                }
            }
        }
        match flattened.len() {
            0 => Type::Never,
            1 => flattened.pop().unwrap(),
            _ => Type::Union(flattened),
        }
    }

    /// Create a simple Callable type
    pub fn callable(params: Vec<Type>, ret: Type) -> Self {
        let params = params
            .into_iter()
            .enumerate()
            .map(|(i, ty)| Param {
                name: format!("_{}", i),
                ty,
                has_default: false,
                kind: ParamKind::Positional,
            })
            .collect();
        Type::Callable {
            params,
            ret: Box::new(ret),
        }
    }

    /// Check if this type is a subtype of Any
    pub fn is_any(&self) -> bool {
        matches!(self, Type::Any)
    }

    /// Check if this type is Unknown
    pub fn is_unknown(&self) -> bool {
        matches!(self, Type::Unknown)
    }

    /// Check if this type is an error
    pub fn is_error(&self) -> bool {
        matches!(self, Type::Error)
    }

    /// Check if this type contains None
    pub fn contains_none(&self) -> bool {
        match self {
            Type::None => true,
            Type::Optional(_) => true,
            Type::Union(types) => types.iter().any(|t| t.contains_none()),
            _ => false,
        }
    }

    /// Remove None from this type
    pub fn without_none(&self) -> Type {
        match self {
            Type::None => Type::Never,
            Type::Optional(inner) => (**inner).clone(),
            Type::Union(types) => {
                let filtered: Vec<_> = types
                    .iter()
                    .filter(|t| !matches!(t, Type::None))
                    .cloned()
                    .collect();
                Type::union(filtered)
            }
            other => other.clone(),
        }
    }

    /// Substitute type variables with concrete types
    pub fn substitute(&self, substitutions: &std::collections::HashMap<TypeVarId, Type>) -> Type {
        match self {
            Type::TypeVar { id, .. } => {
                substitutions.get(id).cloned().unwrap_or_else(|| self.clone())
            }
            Type::List(elem) => Type::List(Box::new(elem.substitute(substitutions))),
            Type::Dict(k, v) => Type::Dict(
                Box::new(k.substitute(substitutions)),
                Box::new(v.substitute(substitutions)),
            ),
            Type::Set(elem) => Type::Set(Box::new(elem.substitute(substitutions))),
            Type::Tuple(elems) => {
                Type::Tuple(elems.iter().map(|t| t.substitute(substitutions)).collect())
            }
            Type::Optional(inner) => Type::Optional(Box::new(inner.substitute(substitutions))),
            Type::Union(types) => {
                Type::union(types.iter().map(|t| t.substitute(substitutions)).collect())
            }
            Type::Intersection(types) => Type::Intersection(
                types.iter().map(|t| t.substitute(substitutions)).collect(),
            ),
            Type::Callable { params, ret } => Type::Callable {
                params: params
                    .iter()
                    .map(|p| Param {
                        name: p.name.clone(),
                        ty: p.ty.substitute(substitutions),
                        has_default: p.has_default,
                        kind: p.kind,
                    })
                    .collect(),
                ret: Box::new(ret.substitute(substitutions)),
            },
            Type::Instance {
                name,
                module,
                type_args,
            } => Type::Instance {
                name: name.clone(),
                module: module.clone(),
                type_args: type_args
                    .iter()
                    .map(|t| t.substitute(substitutions))
                    .collect(),
            },
            // Other types are unchanged
            other => other.clone(),
        }
    }

    /// Collect all type variables in this type
    pub fn type_vars(&self) -> Vec<TypeVarId> {
        let mut vars = Vec::new();
        self.collect_type_vars(&mut vars);
        vars
    }

    fn collect_type_vars(&self, vars: &mut Vec<TypeVarId>) {
        match self {
            Type::TypeVar { id, .. } => {
                if !vars.contains(id) {
                    vars.push(*id);
                }
            }
            Type::List(elem) | Type::Set(elem) | Type::Optional(elem) => {
                elem.collect_type_vars(vars);
            }
            Type::Dict(k, v) => {
                k.collect_type_vars(vars);
                v.collect_type_vars(vars);
            }
            Type::Tuple(elems) | Type::Union(elems) | Type::Intersection(elems) => {
                for elem in elems {
                    elem.collect_type_vars(vars);
                }
            }
            Type::Callable { params, ret } => {
                for param in params {
                    param.ty.collect_type_vars(vars);
                }
                ret.collect_type_vars(vars);
            }
            Type::Instance { type_args, .. } => {
                for arg in type_args {
                    arg.collect_type_vars(vars);
                }
            }
            _ => {}
        }
    }

    /// Unify this type (pattern) with a concrete type, collecting TypeVar bindings
    /// Returns Some(substitutions) on success, None on failure
    pub fn unify(&self, concrete: &Type, subs: &mut HashMap<TypeVarId, Type>) -> bool {
        match (self, concrete) {
            // TypeVar: bind it to the concrete type
            (Type::TypeVar { id, .. }, _) => {
                if let Some(existing) = subs.get(id) {
                    // Already bound - check consistency
                    existing == concrete
                } else {
                    subs.insert(*id, concrete.clone());
                    true
                }
            }

            // Same primitive types
            (Type::Never, Type::Never)
            | (Type::None, Type::None)
            | (Type::Bool, Type::Bool)
            | (Type::Int, Type::Int)
            | (Type::Float, Type::Float)
            | (Type::Str, Type::Str)
            | (Type::Bytes, Type::Bytes)
            | (Type::Any, _)
            | (_, Type::Any)
            | (Type::Unknown, _)
            | (_, Type::Unknown) => true,

            // Numeric widening: int -> float
            (Type::Float, Type::Int) => true,

            // Container types: unify element types
            (Type::List(a), Type::List(b)) => a.unify(b, subs),
            (Type::Set(a), Type::Set(b)) => a.unify(b, subs),
            (Type::Optional(a), Type::Optional(b)) => a.unify(b, subs),
            (Type::Optional(a), b) if !matches!(b, Type::None) => a.unify(b, subs),

            (Type::Dict(k1, v1), Type::Dict(k2, v2)) => {
                k1.unify(k2, subs) && v1.unify(v2, subs)
            }

            (Type::Tuple(a), Type::Tuple(b)) if a.len() == b.len() => {
                a.iter().zip(b.iter()).all(|(t1, t2)| t1.unify(t2, subs))
            }

            // Callable: unify params and return
            (
                Type::Callable { params: p1, ret: r1 },
                Type::Callable { params: p2, ret: r2 },
            ) if p1.len() == p2.len() => {
                let params_ok = p1
                    .iter()
                    .zip(p2.iter())
                    .all(|(a, b)| a.ty.unify(&b.ty, subs));
                params_ok && r1.unify(r2, subs)
            }

            // Instance types: same name, unify type args
            (
                Type::Instance { name: n1, type_args: a1, .. },
                Type::Instance { name: n2, type_args: a2, .. },
            ) if n1 == n2 && a1.len() == a2.len() => {
                a1.iter().zip(a2.iter()).all(|(t1, t2)| t1.unify(t2, subs))
            }

            // Union: concrete must unify with at least one member
            (Type::Union(types), concrete) => {
                types.iter().any(|t| {
                    let mut temp_subs = subs.clone();
                    if t.unify(concrete, &mut temp_subs) {
                        *subs = temp_subs;
                        true
                    } else {
                        false
                    }
                })
            }

            // Same type exactly
            (a, b) if a == b => true,

            _ => false,
        }
    }

    /// Create a TypeVar
    pub fn type_var(id: usize, name: &str) -> Type {
        Type::TypeVar {
            id: TypeVarId(id),
            name: name.to_string(),
            bound: None,
            constraints: vec![],
        }
    }

    /// Create a TypeVar with a bound
    pub fn type_var_bounded(id: usize, name: &str, bound: Type) -> Type {
        Type::TypeVar {
            id: TypeVarId(id),
            name: name.to_string(),
            bound: Some(Box::new(bound)),
            constraints: vec![],
        }
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Never => write!(f, "Never"),
            Type::None => write!(f, "None"),
            Type::Bool => write!(f, "bool"),
            Type::Int => write!(f, "int"),
            Type::Float => write!(f, "float"),
            Type::Str => write!(f, "str"),
            Type::Bytes => write!(f, "bytes"),

            Type::List(elem) => write!(f, "list[{}]", elem),
            Type::Dict(k, v) => write!(f, "dict[{}, {}]", k, v),
            Type::Set(elem) => write!(f, "set[{}]", elem),
            Type::Tuple(elems) => {
                write!(f, "tuple[")?;
                for (i, elem) in elems.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", elem)?;
                }
                write!(f, "]")
            }

            Type::Optional(inner) => write!(f, "{} | None", inner),
            Type::Union(types) => {
                for (i, ty) in types.iter().enumerate() {
                    if i > 0 {
                        write!(f, " | ")?;
                    }
                    write!(f, "{}", ty)?;
                }
                Ok(())
            }
            Type::Intersection(types) => {
                for (i, ty) in types.iter().enumerate() {
                    if i > 0 {
                        write!(f, " & ")?;
                    }
                    write!(f, "{}", ty)?;
                }
                Ok(())
            }

            Type::Callable { params, ret } => {
                write!(f, "(")?;
                for (i, param) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", param.ty)?;
                }
                write!(f, ") -> {}", ret)
            }

            Type::Instance { name, type_args, .. } => {
                write!(f, "{}", name)?;
                if !type_args.is_empty() {
                    write!(f, "[")?;
                    for (i, arg) in type_args.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{}", arg)?;
                    }
                    write!(f, "]")?;
                }
                Ok(())
            }
            Type::ClassType { name, .. } => write!(f, "type[{}]", name),

            Type::Protocol { name, members, .. } => {
                write!(f, "Protocol[{}]", name)?;
                if !members.is_empty() {
                    write!(f, "{{")?;
                    for (i, (member_name, _)) in members.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{}", member_name)?;
                    }
                    write!(f, "}}")?;
                }
                Ok(())
            }

            Type::TypedDict { name, fields, .. } => {
                write!(f, "TypedDict[{}]", name)?;
                if !fields.is_empty() {
                    write!(f, "{{")?;
                    for (i, (field_name, field_ty, required)) in fields.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        if *required {
                            write!(f, "{}: {}", field_name, field_ty)?;
                        } else {
                            write!(f, "{}?: {}", field_name, field_ty)?;
                        }
                    }
                    write!(f, "}}")?;
                }
                Ok(())
            }

            Type::TypeVar { name, .. } => write!(f, "{}", name),

            Type::Any => write!(f, "Any"),
            Type::Unknown => write!(f, "Unknown"),
            Type::Literal(lit) => match lit {
                LiteralValue::Int(n) => write!(f, "Literal[{}]", n),
                LiteralValue::Float(n) => write!(f, "Literal[{}]", n),
                LiteralValue::Str(s) => write!(f, "Literal[\"{}\"]", s),
                LiteralValue::Bool(b) => write!(f, "Literal[{}]", b),
                LiteralValue::None => write!(f, "Literal[None]"),
            },
            Type::SelfType => write!(f, "Self"),

            Type::Error => write!(f, "<error>"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_display() {
        assert_eq!(Type::Int.to_string(), "int");
        assert_eq!(Type::optional(Type::Str).to_string(), "str | None");
        assert_eq!(Type::list(Type::Int).to_string(), "list[int]");
        assert_eq!(
            Type::dict(Type::Str, Type::Int).to_string(),
            "dict[str, int]"
        );
    }

    #[test]
    fn test_union_flattening() {
        let union = Type::union(vec![
            Type::Int,
            Type::Union(vec![Type::Str, Type::Float]),
            Type::Int, // duplicate
        ]);

        match union {
            Type::Union(types) => {
                assert_eq!(types.len(), 3);
                assert!(types.contains(&Type::Int));
                assert!(types.contains(&Type::Str));
                assert!(types.contains(&Type::Float));
            }
            _ => panic!("Expected Union"),
        }
    }

    #[test]
    fn test_without_none() {
        let optional = Type::optional(Type::Str);
        assert_eq!(optional.without_none(), Type::Str);

        let union = Type::Union(vec![Type::Int, Type::None, Type::Str]);
        let without = union.without_none();
        match without {
            Type::Union(types) => {
                assert_eq!(types.len(), 2);
                assert!(!types.contains(&Type::None));
            }
            _ => panic!("Expected Union"),
        }
    }

    #[test]
    fn test_type_var_substitution() {
        use std::collections::HashMap;

        // Create a generic type: List[T]
        let t = Type::type_var(0, "T");
        let list_t = Type::list(t);

        // Substitute T -> Int
        let mut subs = HashMap::new();
        subs.insert(TypeVarId(0), Type::Int);

        let result = list_t.substitute(&subs);
        assert_eq!(result, Type::list(Type::Int));
    }

    #[test]
    fn test_type_vars_collection() {
        // Create Dict[K, V]
        let k = Type::type_var(0, "K");
        let v = Type::type_var(1, "V");
        let dict_kv = Type::dict(k, v);

        let vars = dict_kv.type_vars();
        assert_eq!(vars.len(), 2);
        assert!(vars.contains(&TypeVarId(0)));
        assert!(vars.contains(&TypeVarId(1)));
    }

    #[test]
    fn test_nested_substitution() {
        use std::collections::HashMap;

        // Create Optional[List[T]]
        let t = Type::type_var(0, "T");
        let list_t = Type::list(t);
        let optional_list_t = Type::optional(list_t);

        // Substitute T -> Str
        let mut subs = HashMap::new();
        subs.insert(TypeVarId(0), Type::Str);

        let result = optional_list_t.substitute(&subs);
        assert_eq!(result, Type::optional(Type::list(Type::Str)));
    }

    #[test]
    fn test_callable_substitution() {
        use std::collections::HashMap;

        // Create Callable[[T], T]
        let t = Type::type_var(0, "T");
        let callable = Type::Callable {
            params: vec![Param {
                name: "x".to_string(),
                ty: t.clone(),
                has_default: false,
                kind: ParamKind::Positional,
            }],
            ret: Box::new(t),
        };

        // Substitute T -> Int
        let mut subs = HashMap::new();
        subs.insert(TypeVarId(0), Type::Int);

        let result = callable.substitute(&subs);
        match result {
            Type::Callable { params, ret } => {
                assert_eq!(params[0].ty, Type::Int);
                assert_eq!(*ret, Type::Int);
            }
            _ => panic!("Expected Callable"),
        }
    }

    #[test]
    fn test_unify_type_var() {
        use std::collections::HashMap;

        // Unify T with int -> T=int
        let t = Type::type_var(0, "T");
        let mut subs = HashMap::new();

        assert!(t.unify(&Type::Int, &mut subs));
        assert_eq!(subs.get(&TypeVarId(0)), Some(&Type::Int));
    }

    #[test]
    fn test_unify_list() {
        use std::collections::HashMap;

        // Unify list[T] with list[str] -> T=str
        let t = Type::type_var(0, "T");
        let list_t = Type::list(t);
        let mut subs = HashMap::new();

        assert!(list_t.unify(&Type::list(Type::Str), &mut subs));
        assert_eq!(subs.get(&TypeVarId(0)), Some(&Type::Str));
    }

    #[test]
    fn test_unify_dict() {
        use std::collections::HashMap;

        // Unify dict[K, V] with dict[str, int] -> K=str, V=int
        let k = Type::type_var(0, "K");
        let v = Type::type_var(1, "V");
        let dict_kv = Type::dict(k, v);
        let mut subs = HashMap::new();

        assert!(dict_kv.unify(&Type::dict(Type::Str, Type::Int), &mut subs));
        assert_eq!(subs.get(&TypeVarId(0)), Some(&Type::Str));
        assert_eq!(subs.get(&TypeVarId(1)), Some(&Type::Int));
    }

    #[test]
    fn test_unify_consistency() {
        use std::collections::HashMap;

        // Unify T with int, then T with int again -> ok
        let t = Type::type_var(0, "T");
        let mut subs = HashMap::new();

        assert!(t.unify(&Type::Int, &mut subs));
        assert!(t.unify(&Type::Int, &mut subs)); // Same binding, ok

        // Unify T with str after binding to int -> fail
        assert!(!t.unify(&Type::Str, &mut subs));
    }

    #[test]
    fn test_protocol_display() {
        let protocol = Type::Protocol {
            name: "Sized".to_string(),
            module: Some("typing".to_string()),
            members: vec![
                ("__len__".to_string(), Type::callable(vec![], Type::Int)),
            ],
        };
        assert_eq!(protocol.to_string(), "Protocol[Sized]{__len__}");

        // Empty protocol
        let empty_proto = Type::Protocol {
            name: "Empty".to_string(),
            module: None,
            members: vec![],
        };
        assert_eq!(empty_proto.to_string(), "Protocol[Empty]");
    }

    #[test]
    fn test_literal_display() {
        assert_eq!(
            Type::Literal(LiteralValue::Int(42)).to_string(),
            "Literal[42]"
        );
        assert_eq!(
            Type::Literal(LiteralValue::Str("hello".to_string())).to_string(),
            "Literal[\"hello\"]"
        );
        assert_eq!(
            Type::Literal(LiteralValue::Bool(true)).to_string(),
            "Literal[true]"
        );
        assert_eq!(
            Type::Literal(LiteralValue::None).to_string(),
            "Literal[None]"
        );
    }

    #[test]
    fn test_typed_dict_display() {
        let td = Type::TypedDict {
            name: "Person".to_string(),
            fields: vec![
                ("name".to_string(), Type::Str, true),
                ("age".to_string(), Type::Int, true),
                ("email".to_string(), Type::Str, false), // optional
            ],
            total: true,
        };
        assert_eq!(
            td.to_string(),
            "TypedDict[Person]{name: str, age: int, email?: str}"
        );

        // Empty TypedDict
        let empty_td = Type::TypedDict {
            name: "Empty".to_string(),
            fields: vec![],
            total: true,
        };
        assert_eq!(empty_td.to_string(), "TypedDict[Empty]");
    }
}
