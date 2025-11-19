/// Test: JSON Serialization
///
/// RustScript should support JSON serialization and deserialization
/// with derive macros and manual building.

use json;
use fs;

plugin JsonSerializationTest {
    /// Test struct with Serialize derive
    #[derive(Serialize)]
    struct Template {
        path: Str,
        template: Str,
        bindings: Vec<Str>,
    }

    /// Test struct with both derives
    #[derive(Serialize, Deserialize)]
    struct Config {
        name: Str,
        version: Str,
        features: Vec<Str>,
    }

    /// Test nested serializable structs
    #[derive(Serialize)]
    struct Component {
        name: Str,
        templates: HashMap<Str, Template>,
        props: Vec<Prop>,
    }

    #[derive(Serialize, Deserialize)]
    struct Prop {
        name: Str,
        prop_type: Str,
        optional: bool,
    }

    /// Test basic serialization
    pub fn test_serialize_basic() -> Str {
        let template = Template {
            path: "0.1.2",
            template: "${count}",
            bindings: vec!["count"],
        };

        let json_str = json::to_string(&template).unwrap();
        return json_str;
    }

    /// Test pretty serialization
    pub fn test_serialize_pretty() -> Str {
        let config = Config {
            name: "my-plugin",
            version: "1.0.0",
            features: vec!["hooks", "jsx", "typescript"],
        };

        let json_str = json::to_string_pretty(&config).unwrap();
        return json_str;
    }

    /// Test deserialization
    pub fn test_deserialize() -> Config {
        let json_str = r#"{"name":"test","version":"2.0.0","features":["a","b"]}"#;

        let config: Config = json::from_str(&json_str).unwrap();
        return config;
    }

    /// Test deserialization with error handling
    pub fn test_deserialize_safe(json_str: &Str) -> Option<Config> {
        match json::from_str::<Config>(json_str) {
            Ok(config) => {
                return Some(config);
            }
            Err(_) => {
                return None;
            }
        }
    }

    /// Test serialize with HashMap
    pub fn test_serialize_hashmap() -> Str {
        let mut templates: HashMap<Str, Template> = HashMap::new();

        templates.insert("header", Template {
            path: "0",
            template: "${title}",
            bindings: vec!["title"],
        });

        templates.insert("content", Template {
            path: "1",
            template: "${body}",
            bindings: vec!["body"],
        });

        let component = Component {
            name: "Page",
            templates: templates,
            props: vec![],
        };

        return json::to_string_pretty(&component).unwrap();
    }

    /// Test manual JSON object building
    pub fn test_manual_object() -> Str {
        let mut obj = json::object();

        obj.insert("name", json::string("MyComponent"));
        obj.insert("version", json::number(1));
        obj.insert("enabled", json::boolean(true));
        obj.insert("metadata", json::null());

        return json::stringify(&obj);
    }

    /// Test manual JSON array building
    pub fn test_manual_array() -> Str {
        let mut arr = json::array();

        arr.push(json::string("item1"));
        arr.push(json::string("item2"));
        arr.push(json::number(42));

        return json::stringify(&arr);
    }

    /// Test nested manual building
    pub fn test_manual_nested() -> Str {
        let mut root = json::object();

        // Build nested object
        let mut nested = json::object();
        nested.insert("key", json::string("value"));

        // Build array
        let mut items = json::array();
        items.push(json::number(1));
        items.push(json::number(2));

        root.insert("nested", nested);
        root.insert("items", items);

        return json::stringify_pretty(&root);
    }

    /// Test real-world template JSON generation
    pub fn generate_templates_json(component: &Component) -> Str {
        let mut root = json::object();

        root.insert("componentName", json::string(&component.name));

        // Templates object
        let mut templates_obj = json::object();
        for (key, template) in &component.templates {
            let mut t = json::object();
            t.insert("path", json::string(&template.path));
            t.insert("template", json::string(&template.template));

            let mut bindings = json::array();
            for binding in &template.bindings {
                bindings.push(json::string(binding));
            }
            t.insert("bindings", bindings);

            templates_obj.insert(key, t);
        }
        root.insert("templates", templates_obj);

        // Props array
        let mut props_arr = json::array();
        for prop in &component.props {
            let mut p = json::object();
            p.insert("name", json::string(&prop.name));
            p.insert("type", json::string(&prop.prop_type));
            p.insert("optional", json::boolean(prop.optional));
            props_arr.push(p);
        }
        root.insert("props", props_arr);

        return json::stringify_pretty(&root);
    }

    /// Test file I/O with JSON
    pub fn save_config(config: &Config, path: &Str) {
        let json_str = json::to_string_pretty(config).unwrap();
        fs::write(path, &json_str).unwrap();
    }

    /// Test load config from file
    pub fn load_config(path: &Str) -> Option<Config> {
        if !fs::exists(path) {
            return None;
        }

        let content = fs::read_to_string(path).unwrap();
        match json::from_str::<Config>(&content) {
            Ok(config) => Some(config),
            Err(_) => None,
        }
    }

    /// Test in visitor context
    pub fn visit_program(node: &Program) {
        let mut components: Vec<Component> = vec![];

        // Process components...
        for item in &node.body {
            if matches!(item, FunctionDeclaration) {
                let component = Component {
                    name: item.id.name.clone(),
                    templates: HashMap::new(),
                    props: vec![],
                };
                components.push(component);
            }
        }

        // Write template JSON for each component
        for comp in &components {
            let json_str = json::to_string_pretty(&comp).unwrap();
            let path = format!("output/{}.templates.json", comp.name);
            fs::write(&path, &json_str).unwrap();
        }
    }
}
