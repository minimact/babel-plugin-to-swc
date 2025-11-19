use swc_ecma_ast::*;
use crate::component::*;
use crate::MinimactTransformer;
use crate::ParentContext;

/// Extract useState hook
///
/// Pattern: const [value, setValue] = useState(initialValue)
pub fn extract_use_state(
    call: &CallExpr,
    component: &mut Component,
    transformer: &MinimactTransformer
) {
    // Get parent context - must be a VarDeclarator with ArrayPattern
    let (_var_name, _setter_name): (String, String) = match transformer.get_parent() {
        Some(ParentContext::VarDeclarator(_)) => {
            // Need to get the actual array pattern from the VarDeclarator
            // This requires access to the actual AST node
            // For now, return placeholder
            return;
        }
        _ => return,
    };
}

/// Extract useEffect hook
pub fn extract_use_effect(
    call: &CallExpr,
    component: &mut Component,
    _transformer: &MinimactTransformer
) {
    // Extract dependencies from second argument
    let dependencies = if call.args.len() > 1 {
        if let Some(arg) = call.args.get(1) {
            extract_dependency_array(&arg.expr)
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    // Check if callback uses client-side APIs
    let is_client_side = false; // TODO: Analyze callback body

    component.use_effect.push(UseEffectInfo {
        dependencies,
        is_client_side,
    });
}

/// Extract useRef hook
pub fn extract_use_ref(
    call: &CallExpr,
    component: &mut Component,
    transformer: &MinimactTransformer
) {
    // Get parent context - must be a VarDeclarator
    let ref_name = match transformer.get_parent() {
        Some(ParentContext::VarDeclarator(name)) => name.clone(),
        _ => return,
    };

    // Get initial value from first argument
    let initial_value = call.args.get(0)
        .map(|arg| generate_csharp_expression(&arg.expr))
        .unwrap_or_else(|| "null".to_string());

    component.use_ref.push(UseRefInfo {
        name: ref_name,
        initial_value,
    });
}

/// Extract useClientState hook
pub fn extract_use_client_state(
    call: &CallExpr,
    component: &mut Component,
    transformer: &MinimactTransformer
) {
    // Similar to useState but tracked separately
    let (_var_name, _setter_name): (String, String) = match transformer.get_parent() {
        Some(ParentContext::VarDeclarator(_)) => {
            // Need array pattern extraction
            return;
        }
        _ => return,
    };
}

/// Extract useMarkdown hook
pub fn extract_use_markdown(
    call: &CallExpr,
    component: &mut Component,
    transformer: &MinimactTransformer
) {
    // Pattern: const [content, setContent] = useMarkdown(initial)
    // Similar extraction to useState
}

/// Extract custom hook call
pub fn extract_custom_hook(
    call: &CallExpr,
    component: &mut Component,
    hook_name: &str,
    transformer: &MinimactTransformer
) {
    // Get the variable name(s) from parent context
    let instance_name = match transformer.get_parent() {
        Some(ParentContext::VarDeclarator(name)) => name.clone(),
        _ => return,
    };

    // Generate class name from hook name
    let class_name = hook_name_to_class_name(hook_name);

    component.custom_hooks.push(CustomHookInstance {
        hook_name: hook_name.to_string(),
        instance_name,
        class_name,
        return_values: Vec::new(),
    });
}

/// Convert hook name to class name (useCounter -> CounterHook)
fn hook_name_to_class_name(hook_name: &str) -> String {
    let without_use = hook_name.strip_prefix("use").unwrap_or(hook_name);
    format!("{}Hook", without_use)
}

/// Extract dependency array from expression
fn extract_dependency_array(expr: &Expr) -> Vec<String> {
    match expr {
        Expr::Array(arr) => {
            arr.elems.iter()
                .filter_map(|elem| {
                    elem.as_ref().map(|e| {
                        match &*e.expr {
                            Expr::Ident(ident) => ident.sym.to_string(),
                            _ => String::new(),
                        }
                    })
                })
                .filter(|s| !s.is_empty())
                .collect()
        }
        _ => Vec::new(),
    }
}

/// Generate C# expression from AST
fn generate_csharp_expression(expr: &Expr) -> String {
    match expr {
        Expr::Lit(lit) => match lit {
            Lit::Str(s) => format!("\"{}\"", String::from_utf8_lossy(s.value.as_bytes())),
            Lit::Num(n) => n.value.to_string(),
            Lit::Bool(b) => b.value.to_string(),
            Lit::Null(_) => "null".to_string(),
            _ => "null".to_string(),
        },
        Expr::Ident(ident) => ident.sym.to_string(),
        Expr::Array(_) => "new List<dynamic>()".to_string(),
        Expr::Object(_) => "new Dictionary<string, dynamic>()".to_string(),
        _ => "null".to_string(),
    }
}
