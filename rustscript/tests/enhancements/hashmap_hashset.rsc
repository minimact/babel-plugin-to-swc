/// Test: HashMap and HashSet Support
///
/// RustScript should support HashMap<K, V> and HashSet<T> with full
/// collection operations.

plugin HashMapHashSetTest {
    struct Template {
        path: Str,
        bindings: Vec<Str>,
    }

    struct Component {
        name: Str,
        templates: HashMap<Str, Template>,
        state_types: HashMap<Str, Str>,
        external_imports: HashSet<Str>,
    }

    /// Test HashMap operations
    pub fn test_hashmap_basic() {
        let mut map: HashMap<Str, Str> = HashMap::new();

        // Insert
        map.insert("key1", "value1");
        map.insert("key2", "value2");

        // Get
        let value = map.get(&"key1");
        if let Some(v) = value {
            let s = v.clone();
        }

        // Contains
        if map.contains_key(&"key1") {
            // ...
        }

        // Remove
        let removed = map.remove(&"key2");

        // Length
        let count = map.len();
        let empty = map.is_empty();

        // Iteration
        for (key, value) in &map {
            let k = key.clone();
            let v = value.clone();
        }

        // Keys and values
        for key in map.keys() {
            let k = key.clone();
        }

        for value in map.values() {
            let v = value.clone();
        }
    }

    /// Test HashSet operations
    pub fn test_hashset_basic() {
        let mut set: HashSet<Str> = HashSet::new();

        // Insert
        set.insert("item1");
        set.insert("item2");
        set.insert("item1"); // Duplicate, should not add

        // Contains
        if set.contains(&"item1") {
            // ...
        }

        // Remove
        set.remove(&"item2");

        // Length
        let count = set.len();
        let empty = set.is_empty();

        // Iteration
        for item in &set {
            let i = item.clone();
        }
    }

    /// Test HashMap with struct values
    pub fn test_hashmap_struct() {
        let mut templates: HashMap<Str, Template> = HashMap::new();

        let template = Template {
            path: "0.1.2",
            bindings: vec!["count", "name"],
        };

        templates.insert("header", template);

        // Get and modify
        if let Some(t) = templates.get_mut(&"header") {
            t.bindings.push("extra");
        }

        // Get or insert default
        let entry = templates.get(&"footer").unwrap_or(&Template {
            path: "",
            bindings: vec![],
        });
    }

    /// Test in real visitor context
    pub fn visit_jsx_element(jsx: &JSXElement) {
        let mut component = Component {
            name: "Test",
            templates: HashMap::new(),
            state_types: HashMap::new(),
            external_imports: HashSet::new(),
        };

        // Build path key
        let path = "0.1.2";

        // Check if template exists
        if !component.templates.contains_key(&path) {
            let template = Template {
                path: path.clone(),
                bindings: vec![],
            };
            component.templates.insert(path.clone(), template);
        }

        // Get and update
        if let Some(t) = component.templates.get_mut(&path) {
            // Extract bindings from JSX attributes
            for attr in &jsx.opening.attrs {
                if matches!(attr, JSXAttribute) {
                    t.bindings.push(attr.name.name.clone());
                }
            }
        }

        // Track external import
        component.external_imports.insert("react");

        // Check membership
        if component.external_imports.contains(&"lodash") {
            // ...
        }
    }

    /// Test nested HashMap
    pub fn test_nested_hashmap() {
        let mut nested: HashMap<Str, HashMap<Str, Str>> = HashMap::new();

        let mut inner: HashMap<Str, Str> = HashMap::new();
        inner.insert("a", "1");
        inner.insert("b", "2");

        nested.insert("outer", inner);

        if let Some(inner_map) = nested.get(&"outer") {
            if let Some(value) = inner_map.get(&"a") {
                let v = value.clone();
            }
        }
    }
}
