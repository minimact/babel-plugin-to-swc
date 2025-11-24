// Test: init() function should return State
writer TestInit {
    struct State {
        count: i32,
        name: Str,
    }

    fn init() -> State {
        State {
            count: 0,
            name: String::new(),
        }
    }
}
