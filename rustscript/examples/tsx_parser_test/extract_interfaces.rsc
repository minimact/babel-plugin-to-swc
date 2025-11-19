/// Extract interface property names from TSX
///
/// This plugin demonstrates RustScript's ability to:
/// 1. Visit TSInterfaceDeclaration nodes
/// 2. Extract property names
/// 3. Return a list of strings

plugin InterfaceExtractor {
    /// Main visitor function for interface declarations
    /// Returns a list of property names from the interface
    pub fn visit_interface_declaration(node: &TSInterfaceDeclaration) -> Vec<Str> {
        let mut properties: Vec<Str> = vec![];

        // Get interface name
        let interface_name = node.id.name.clone();
        properties.push(interface_name);

        // Iterate over interface body members
        for member in &node.body.body {
            // Check if this is a property signature
            if matches!(member, TSPropertySignature) {
                let prop_name = member.key.name.clone();
                properties.push(prop_name);
            }
        }

        return properties;
    }

    /// Extract useState call information
    /// Returns the callee name if it's a useState call with type args
    pub fn visit_call_expression(node: &CallExpression) -> Vec<Str> {
        let mut results: Vec<Str> = vec![];

        // Check if this is a useState call
        if matches!(node.callee, Identifier) {
            let callee_name = node.callee.name.clone();
            if callee_name == "useState" {
                results.push(callee_name);

                // Check for type arguments
                if node.type_args.len() > 0 {
                    results.push("has_type_arg");
                }
            }
        }

        return results;
    }
}
