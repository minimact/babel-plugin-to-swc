use std::collections::{HashMap, HashSet};
use swc_ecma_ast::*;

/// Represents a React component
#[derive(Clone, Debug)]
pub struct Component {
    pub name: String,
    pub props: Vec<Prop>,
    // State hooks
    pub use_state: Vec<UseStateInfo>,
    pub use_client_state: Vec<UseStateInfo>,
    pub use_protected_state: Vec<UseStateInfo>,
    pub use_state_x: Vec<UseStateXInfo>,
    // Effect and ref hooks
    pub use_effect: Vec<UseEffectInfo>,
    pub use_ref: Vec<UseRefInfo>,
    // Content hooks
    pub use_markdown: Vec<UseMarkdownInfo>,
    pub use_razor_markdown: Vec<UseRazorMarkdownInfo>,
    pub use_template: Option<UseTemplateInfo>,
    // UI state hooks
    pub use_validation: Vec<UseValidationInfo>,
    pub use_modal: Vec<UseModalInfo>,
    pub use_toggle: Vec<UseToggleInfo>,
    pub use_dropdown: Vec<UseDropdownInfo>,
    // Pub/Sub hooks
    pub use_pub: Vec<UsePubInfo>,
    pub use_sub: Vec<UseSubInfo>,
    // Task scheduling hooks
    pub use_micro_task: Vec<UseMicroTaskInfo>,
    pub use_macro_task: Vec<UseMacroTaskInfo>,
    // Server communication hooks
    pub use_signalr: Vec<UseSignalRInfo>,
    pub use_server_task: Vec<UseServerTaskInfo>,
    pub paginated_tasks: Vec<PaginatedTaskInfo>,
    // MVC integration hooks
    pub use_mvc_state: Vec<UseMvcStateInfo>,
    pub use_mvc_view_model: Vec<UseMvcViewModelInfo>,
    // Optimization hooks
    pub use_predict_hint: Vec<UsePredictHintInfo>,
    // Custom hooks
    pub custom_hooks: Vec<CustomHookInstance>,
    pub imported_hook_metadata: HashMap<String, HookMetadata>,
    // Handlers and effects
    pub event_handlers: Vec<EventHandler>,
    pub client_handlers: Vec<ClientHandler>,
    pub client_effects: Vec<ClientEffect>,
    // Variables and functions
    pub local_variables: Vec<LocalVariable>,
    pub helper_functions: Vec<HelperFunction>,
    pub render_body: Option<Box<Expr>>,
    // Plugin and state tracking
    pub plugin_usages: Vec<PluginUsage>,
    pub state_types: HashMap<String, String>,
    pub dependencies: HashMap<String, Vec<String>>,
    pub external_imports: HashSet<String>,
    pub client_computed_vars: HashSet<String>,
    // Templates
    pub templates: HashMap<String, Template>,
    pub loop_templates: Vec<LoopTemplate>,
    pub structural_templates: Vec<StructuralTemplate>,
    pub conditional_element_templates: HashMap<String, ConditionalElementTemplate>,
    pub expression_templates: Vec<ExpressionTemplate>,
}

impl Component {
    pub fn new(name: String) -> Self {
        Self {
            name,
            props: Vec::new(),
            // State hooks
            use_state: Vec::new(),
            use_client_state: Vec::new(),
            use_protected_state: Vec::new(),
            use_state_x: Vec::new(),
            // Effect and ref hooks
            use_effect: Vec::new(),
            use_ref: Vec::new(),
            // Content hooks
            use_markdown: Vec::new(),
            use_razor_markdown: Vec::new(),
            use_template: None,
            // UI state hooks
            use_validation: Vec::new(),
            use_modal: Vec::new(),
            use_toggle: Vec::new(),
            use_dropdown: Vec::new(),
            // Pub/Sub hooks
            use_pub: Vec::new(),
            use_sub: Vec::new(),
            // Task scheduling hooks
            use_micro_task: Vec::new(),
            use_macro_task: Vec::new(),
            // Server communication hooks
            use_signalr: Vec::new(),
            use_server_task: Vec::new(),
            paginated_tasks: Vec::new(),
            // MVC integration hooks
            use_mvc_state: Vec::new(),
            use_mvc_view_model: Vec::new(),
            // Optimization hooks
            use_predict_hint: Vec::new(),
            // Custom hooks
            custom_hooks: Vec::new(),
            imported_hook_metadata: HashMap::new(),
            // Handlers and effects
            event_handlers: Vec::new(),
            client_handlers: Vec::new(),
            client_effects: Vec::new(),
            // Variables and functions
            local_variables: Vec::new(),
            helper_functions: Vec::new(),
            render_body: None,
            // Plugin and state tracking
            plugin_usages: Vec::new(),
            state_types: HashMap::new(),
            dependencies: HashMap::new(),
            external_imports: HashSet::new(),
            client_computed_vars: HashSet::new(),
            // Templates
            templates: HashMap::new(),
            loop_templates: Vec::new(),
            structural_templates: Vec::new(),
            conditional_element_templates: HashMap::new(),
            expression_templates: Vec::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Prop {
    pub name: String,
    pub prop_type: String,
}

#[derive(Clone, Debug)]
pub struct UseStateInfo {
    pub var_name: String,
    pub setter_name: Option<String>,
    pub initial_value: String,
    pub state_type: String,
    pub is_client_state: bool,
}

#[derive(Clone, Debug)]
pub struct UseStateXInfo {
    pub var_name: String,
    pub selector: String,
}

#[derive(Clone, Debug)]
pub struct UseEffectInfo {
    pub dependencies: Vec<String>,
    pub is_client_side: bool,
}

#[derive(Clone, Debug)]
pub struct UseRefInfo {
    pub name: String,
    pub initial_value: String,
}

#[derive(Clone, Debug)]
pub struct UseMarkdownInfo {
    pub name: String,
    pub setter: String,
    pub initial_value: String,
}

#[derive(Clone, Debug)]
pub struct UseTemplateInfo {
    pub template_name: String,
}

#[derive(Clone, Debug)]
pub struct UseValidationInfo {
    pub name: String,
    pub rules: Vec<ValidationRule>,
}

#[derive(Clone, Debug)]
pub struct ValidationRule {
    pub field: String,
    pub rule_type: String,
    pub message: String,
}

#[derive(Clone, Debug)]
pub struct UseModalInfo {
    pub name: String,
    pub is_open_var: String,
    pub open_fn: String,
    pub close_fn: String,
}

#[derive(Clone, Debug)]
pub struct UseToggleInfo {
    pub name: String,
    pub value_var: String,
    pub toggle_fn: String,
}

#[derive(Clone, Debug)]
pub struct UseDropdownInfo {
    pub name: String,
    pub is_open_var: String,
    pub toggle_fn: String,
}

#[derive(Clone, Debug)]
pub struct CustomHookInstance {
    pub hook_name: String,
    pub instance_name: String,
    pub class_name: String,
    pub return_values: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct EventHandler {
    pub name: String,
    pub params: Vec<String>,
    pub is_async: bool,
}

#[derive(Clone, Debug)]
pub struct ClientHandler {
    pub name: String,
    pub body: String,
}

#[derive(Clone, Debug)]
pub struct ClientEffect {
    pub dependencies: Vec<String>,
    pub body: String,
}

#[derive(Clone, Debug)]
pub struct LocalVariable {
    pub name: String,
    pub var_type: String,
    pub initial_value: String,
    pub is_const: bool,
}

#[derive(Clone, Debug)]
pub struct HelperFunction {
    pub name: String,
    pub params: Vec<FunctionParam>,
    pub return_type: String,
    pub is_async: bool,
}

#[derive(Clone, Debug)]
pub struct FunctionParam {
    pub name: String,
    pub param_type: String,
}

#[derive(Clone, Debug)]
pub struct PluginUsage {
    pub plugin_name: String,
    pub state_binding: String,
    pub version: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Template {
    pub path: String,
    pub template: String,
    pub bindings: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct LoopTemplate {
    pub state_key: String,
    pub item_var: String,
    pub index_var: Option<String>,
    pub key_expression: String,
}

#[derive(Clone, Debug)]
pub struct StructuralTemplate {
    pub template_type: String, // "conditional" or "logical"
    pub condition_binding: String,
}

#[derive(Clone, Debug)]
pub struct ConditionalElementTemplate {
    pub path: String,
    pub condition_expression: String,
    pub evaluable: bool,
}

#[derive(Clone, Debug)]
pub struct ExpressionTemplate {
    pub template_type: String,
    pub state_key: String,
    pub binding: String,
    pub method: Option<String>,
    pub args: Vec<String>,
}

// =============================================================================
// Additional Hook Info Structs
// =============================================================================

#[derive(Clone, Debug)]
pub struct UseRazorMarkdownInfo {
    pub name: String,
    pub setter: String,
    pub initial_value: String,
    pub has_razor_syntax: bool,
    pub referenced_variables: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct UsePubInfo {
    pub name: String,
    pub channel: Option<String>,
}

#[derive(Clone, Debug)]
pub struct UseSubInfo {
    pub name: String,
    pub channel: Option<String>,
    pub has_callback: bool,
}

#[derive(Clone, Debug)]
pub struct UseMicroTaskInfo {
    pub body: Option<Box<Expr>>,
}

#[derive(Clone, Debug)]
pub struct UseMacroTaskInfo {
    pub body: Option<Box<Expr>>,
    pub delay: u32,
}

#[derive(Clone, Debug)]
pub struct UseSignalRInfo {
    pub name: String,
    pub hub_url: Option<String>,
    pub has_on_message: bool,
}

#[derive(Clone, Debug)]
pub struct UseServerTaskInfo {
    pub name: String,
    pub async_function: Option<Box<Expr>>,
    pub parameters: Vec<TaskParameter>,
    pub is_streaming: bool,
    pub estimated_chunks: Option<u32>,
    pub return_type: String,
    pub runtime: String,
    pub parallel: bool,
}

#[derive(Clone, Debug)]
pub struct TaskParameter {
    pub name: String,
    pub param_type: String,
}

#[derive(Clone, Debug)]
pub struct PaginatedTaskInfo {
    pub name: String,
    pub fetch_task_name: String,
    pub count_task_name: Option<String>,
    pub page_size: u32,
    pub runtime: String,
    pub parallel: bool,
}

#[derive(Clone, Debug)]
pub struct UseMvcStateInfo {
    pub name: Option<String>,
    pub setter: Option<String>,
    pub property_name: String,
    pub mvc_type: String,
}

#[derive(Clone, Debug)]
pub struct UseMvcViewModelInfo {
    pub name: String,
}

#[derive(Clone, Debug)]
pub struct UsePredictHintInfo {
    pub hint_id: Option<String>,
    pub predicted_state: Option<Box<Expr>>,
}

#[derive(Clone, Debug)]
pub struct HookMetadata {
    pub class_name: String,
    pub states: Vec<UseStateInfo>,
    pub methods: Vec<HelperFunction>,
    pub event_handlers: Vec<EventHandler>,
    pub return_values: Vec<HookReturnValue>,
    pub jsx_elements: Option<Box<Expr>>,
}

#[derive(Clone, Debug)]
pub struct HookReturnValue {
    pub name: String,
    pub value_type: String, // "state", "function", "jsx", etc.
}
