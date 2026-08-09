use std::collections::HashSet;

use cvn::model::{BoolExpr, CmpOp, Expr, Op, Val};

/// Parse a ConcIR branch condition string (e.g. `"count > 0"`) into a CVN `BoolExpr`.
///
/// `enum_variants` provides known enum variant names so that identifiers like `"Init"`
/// are treated as `Lit(Enum("Init"))` rather than `Ref("Init")`.
pub(crate) fn parse_condition(
    cond: &str,
    enum_variants: &HashSet<String>,
) -> Result<BoolExpr, String> {
    let cond = cond.trim();

    // Try two-character operators first, then single-character.
    let cmp_ops: &[(&str, CmpOp)] = &[
        ("==", CmpOp::Eq),
        ("!=", CmpOp::Ne),
        (">=", CmpOp::Ge),
        ("<=", CmpOp::Le),
        (">", CmpOp::Gt),
        ("<", CmpOp::Lt),
    ];

    for &(sym, ref op) in cmp_ops {
        if let Some(pos) = cond.find(sym) {
            let lhs_str = cond[..pos].trim();
            let rhs_str = cond[pos + sym.len()..].trim();
            if lhs_str.is_empty() || rhs_str.is_empty() {
                continue;
            }
            let lhs = parse_expr(lhs_str, enum_variants)?;
            let rhs = parse_expr(rhs_str, enum_variants)?;
            return Ok(BoolExpr::Cmp {
                op: op.clone(),
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            });
        }
    }

    Err(format!("no comparison operator found in condition: '{cond}'"))
}

/// Parse a ConcIR value string into a CVN `Expr`.
///
/// Handles literals (`5`, `true`, `false`), variable references (`count`),
/// enum literals (if the identifier is in `enum_variants`), and simple binary
/// expressions (`count + 1`).
pub(crate) fn parse_expr(
    s: &str,
    enum_variants: &HashSet<String>,
) -> Result<Expr, String> {
    let s = s.trim();
    if s.is_empty() {
        return Err("empty expression".to_string());
    }

    let binops: &[(&str, Op)] = &[
        (" + ", Op::Add),
        (" - ", Op::Sub),
        (" * ", Op::Mul),
        (" / ", Op::Div),
        (" % ", Op::Mod),
    ];

    for &(sym, ref op) in binops {
        if let Some(pos) = s.find(sym) {
            let lhs_str = s[..pos].trim();
            let rhs_str = s[pos + sym.len()..].trim();
            if lhs_str.is_empty() || rhs_str.is_empty() {
                continue;
            }
            let lhs = parse_atom(lhs_str, enum_variants)?;
            let rhs = parse_atom(rhs_str, enum_variants)?;
            return Ok(Expr::BinOp {
                op: op.clone(),
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            });
        }
    }

    parse_atom(s, enum_variants)
}

/// Parse a single atom: literal or variable reference.
fn parse_atom(s: &str, enum_variants: &HashSet<String>) -> Result<Expr, String> {
    let s = s.trim();

    // Boolean literals
    if s == "true" {
        return Ok(Expr::Lit(Val::bool(true)));
    }
    if s == "false" {
        return Ok(Expr::Lit(Val::bool(false)));
    }

    // Integer literal
    if let Ok(i) = s.parse::<i64>() {
        return Ok(Expr::Lit(Val::int(i)));
    }

    // Float literal (must contain '.')
    if s.contains('.') {
        if let Ok(f) = s.parse::<f64>() {
            return Ok(Expr::Lit(Val::float(f)));
        }
    }

    // Quoted string literal
    if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
        let inner = &s[1..s.len() - 1];
        return Ok(Expr::Lit(Val::string(inner)));
    }

    // Known enum variant → Lit(Enum)
    if enum_variants.contains(s) {
        return Ok(Expr::Lit(Val::enum_val(s)));
    }

    // Unknown literal (looks like a plain identifier) → `Unknown` if it starts
    // with uppercase and is not a variable reference context.
    // Default: treat as variable reference.
    if !is_valid_identifier(s) {
        return Err(format!("invalid expression atom: '{s}'"));
    }

    Ok(Expr::Ref(s.to_string()))
}

fn is_valid_identifier(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_alphanumeric() || c == '_')
}

/// Convert a ConcIR `init` JSON value into a CVN `Val`.
pub(crate) fn json_value_to_val(v: &serde_json::Value) -> Val {
    match v {
        serde_json::Value::Bool(b) => Val::bool(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Val::int(i)
            } else if let Some(f) = n.as_f64() {
                Val::float(f)
            } else {
                Val::Unknown
            }
        }
        serde_json::Value::String(s) => {
            // Could be an enum variant or a string value.
            // Caller needs to disambiguate; default to string.
            Val::string(s.clone())
        }
        _ => Val::Unknown,
    }
}

/// Convert a ConcIR `init` JSON value into a CVN `Val`, using enum variant knowledge.
pub(crate) fn json_value_to_val_with_variants(
    v: &serde_json::Value,
    enum_variants: &HashSet<String>,
) -> Val {
    match v {
        serde_json::Value::String(s) if enum_variants.contains(s.as_str()) => {
            Val::enum_val(s.clone())
        }
        _ => json_value_to_val(v),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn no_enums() -> HashSet<String> {
        HashSet::new()
    }

    fn with_enums(vs: &[&str]) -> HashSet<String> {
        vs.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn parse_simple_condition() {
        let g = parse_condition("count > 0", &no_enums()).unwrap();
        assert_eq!(
            g,
            BoolExpr::Cmp {
                op: CmpOp::Gt,
                lhs: Box::new(Expr::Ref("count".into())),
                rhs: Box::new(Expr::Lit(Val::int(0))),
            }
        );
    }

    #[test]
    fn parse_eq_bool_condition() {
        let g = parse_condition("done == true", &no_enums()).unwrap();
        assert_eq!(
            g,
            BoolExpr::Cmp {
                op: CmpOp::Eq,
                lhs: Box::new(Expr::Ref("done".into())),
                rhs: Box::new(Expr::Lit(Val::bool(true))),
            }
        );
    }

    #[test]
    fn parse_lt_condition() {
        let g = parse_condition("i < 10", &no_enums()).unwrap();
        assert_eq!(
            g,
            BoolExpr::Cmp {
                op: CmpOp::Lt,
                lhs: Box::new(Expr::Ref("i".into())),
                rhs: Box::new(Expr::Lit(Val::int(10))),
            }
        );
    }

    #[test]
    fn parse_enum_condition() {
        let enums = with_enums(&["Init", "Running", "Done"]);
        let g = parse_condition("state == Init", &enums).unwrap();
        assert_eq!(
            g,
            BoolExpr::Cmp {
                op: CmpOp::Eq,
                lhs: Box::new(Expr::Ref("state".into())),
                rhs: Box::new(Expr::Lit(Val::enum_val("Init"))),
            }
        );
    }

    #[test]
    fn parse_binop_expr() {
        let e = parse_expr("count + 1", &no_enums()).unwrap();
        assert_eq!(
            e,
            Expr::BinOp {
                op: Op::Add,
                lhs: Box::new(Expr::Ref("count".into())),
                rhs: Box::new(Expr::Lit(Val::int(1))),
            }
        );
    }

    #[test]
    fn parse_literal_int() {
        let e = parse_expr("42", &no_enums()).unwrap();
        assert_eq!(e, Expr::Lit(Val::int(42)));
    }

    #[test]
    fn parse_literal_bool() {
        assert_eq!(parse_expr("true", &no_enums()).unwrap(), Expr::Lit(Val::bool(true)));
        assert_eq!(parse_expr("false", &no_enums()).unwrap(), Expr::Lit(Val::bool(false)));
    }

    #[test]
    fn parse_var_ref() {
        let e = parse_expr("my_var", &no_enums()).unwrap();
        assert_eq!(e, Expr::Ref("my_var".into()));
    }

    #[test]
    fn parse_enum_value() {
        let enums = with_enums(&["Running"]);
        let e = parse_expr("Running", &enums).unwrap();
        assert_eq!(e, Expr::Lit(Val::enum_val("Running")));
    }

    #[test]
    fn parse_sub_expr() {
        let e = parse_expr("count - 1", &no_enums()).unwrap();
        assert_eq!(
            e,
            Expr::BinOp {
                op: Op::Sub,
                lhs: Box::new(Expr::Ref("count".into())),
                rhs: Box::new(Expr::Lit(Val::int(1))),
            }
        );
    }

    #[test]
    fn invalid_condition_no_op() {
        assert!(parse_condition("foobar", &no_enums()).is_err());
    }

    #[test]
    fn json_val_bool() {
        assert_eq!(json_value_to_val(&serde_json::json!(false)), Val::bool(false));
    }

    #[test]
    fn json_val_int() {
        assert_eq!(json_value_to_val(&serde_json::json!(42)), Val::int(42));
    }

    #[test]
    fn json_val_enum_with_variants() {
        let enums = with_enums(&["Init"]);
        assert_eq!(
            json_value_to_val_with_variants(&serde_json::json!("Init"), &enums),
            Val::enum_val("Init"),
        );
    }
}
