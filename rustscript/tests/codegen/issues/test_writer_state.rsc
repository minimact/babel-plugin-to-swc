// Test Issue 8: Missing state fields in SWC struct
writer TestWriterState {
    struct State {
        component_name: Str,
        count: i32,
    }

    fn init() -> State {
        State {
            component_name: String::new(),
            count: 0,
        }
    }

    fn process() {
        self.component_name = "Test".to_string();
        self.count += 1;
    }
}
