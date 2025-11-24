// Test Issue 8: Missing state fields in SWC struct
writer TestWriterState {
    struct State {
        component_name: Str,
    }

    fn init() -> State {
        State {
            component_name: String::new(),
        }
    }

    fn process() {
        self.component_name = "Test".to_string();
    }
}
